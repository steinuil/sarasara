use clap::Parser;

#[derive(Parser)]
struct CliOptions {
    #[clap(long, default_value = "0.0.0.0:8080")]
    pub bind_address: String,

    #[clap(long)]
    pub public_url: surf::Url,

    #[clap(long, default_value = "https://www.raiplaysound.it")]
    pub raiplaysound_url: surf::Url,
}

#[derive(Clone)]
struct State {
    pub raiplaysound_url: surf::Url,
    pub public_url: surf::Url,
}

#[derive(Clone, serde::Deserialize)]
struct Genre {
    pub name: String,
}

#[derive(Clone, serde::Deserialize)]
struct PodcastInfo {
    pub title: String,
    pub description: String,
    pub image: String,
    pub editor: String,
    pub weblink: String,
    pub genres: Vec<Genre>,
    pub subgenres: Vec<Genre>,
}

#[derive(Clone, serde::Deserialize)]
struct TrackInfo {
    pub date: String,
    pub page_url: String,
}

#[derive(Clone, serde::Deserialize)]
struct Audio {
    pub url: String,
    pub duration: String,
}

#[derive(Clone, serde::Deserialize)]
struct Card {
    pub uniquename: String,
    pub toptitle: String,
    pub description: String,
    pub audio: Audio,
    pub image: String,
    pub track_info: TrackInfo,
    pub episode: Option<String>,
    pub season: Option<String>,
}

#[derive(Clone, serde::Deserialize)]
struct Block {
    pub cards: Vec<Card>,
}

#[derive(Clone, serde::Deserialize)]
struct RaiPlayProgram {
    pub podcast_info: PodcastInfo,
    pub block: Block,
}

fn parse_date(date: &str) -> Option<time::OffsetDateTime> {
    if date.len() != 10 {
        return None;
    }

    let year: i32 = date[..4].parse().ok()?;
    let month: u8 = date[5..7].parse().ok()?;
    let day: u8 = date[8..].parse().ok()?;

    Some(
        time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(year, time::Month::try_from(month).ok()?, day).ok()?,
            time::Time::MIDNIGHT,
        )
        .assume_offset(time::UtcOffset::UTC),
    )
}

fn make_url(base_url: &surf::Url, path: &str) -> String {
    let mut url = base_url.clone();
    url.set_path(path);
    url.to_string()
}

fn concat_url_path_with_slash(base: &surf::Url, path: &str) -> surf::Url {
    let mut url = base.clone();
    let base_path = url.path();
    let path = match (base_path.strip_suffix('/'), path.strip_prefix('/')) {
        (Some(base_path), Some(path)) => format!("{base_path}/{path}"),
        (None, None) => format!("{base_path}/{path}"),
        (Some(base_path), None) => format!("{base_path}/{path}"),
        (None, Some(path)) => format!("{base_path}/{path}"),
    };
    url.set_path(&path);
    url
}

fn make_audio_url(base: &surf::Url, url_param: &str) -> String {
    let mut url = concat_url_path_with_slash(base, "/audio");
    // possible bug: what if the URL contains &?
    url.set_query(Some(&format!("url={}", url_param)));
    url.to_string()
}

impl RaiPlayProgram {
    fn into_rss(self, raiplaysound_url: &surf::Url, public_url: &surf::Url) -> rss::Channel {
        rss::Channel {
            title: self.podcast_info.title.clone(),
            link: make_url(raiplaysound_url, &self.podcast_info.weblink),
            description: self.podcast_info.description.clone(),
            managing_editor: Some(self.podcast_info.editor),
            categories: self
                .podcast_info
                .genres
                .into_iter()
                .chain(self.podcast_info.subgenres.into_iter())
                .map(|genre| rss::Category {
                    name: genre.name,
                    domain: None,
                })
                .collect(),
            generator: Some("sarasara https://github.com/steinuil/sarasara".to_string()),
            image: Some(rss::Image {
                url: make_url(raiplaysound_url, &self.podcast_info.image),
                title: self.podcast_info.title,
                description: Some(self.podcast_info.description),
                link: make_url(raiplaysound_url, &self.podcast_info.weblink),
                ..Default::default()
            }),
            items: self
                .block
                .cards
                .into_iter()
                .map(|card| rss::Item {
                    title: Some(card.toptitle),
                    link: Some(make_url(raiplaysound_url, &card.track_info.page_url)),
                    description: Some(card.description),
                    pub_date: parse_date(&card.track_info.date).and_then(|date| {
                        date.format(&time::format_description::well_known::Rfc2822)
                            .ok()
                    }),
                    enclosure: Some(rss::Enclosure {
                        url: make_audio_url(public_url, &card.audio.url),
                        mime_type: "audio/mpeg".to_string(),
                        length: "".to_string(),
                    }),
                    guid: Some(rss::Guid {
                        value: card.uniquename,
                        permalink: false,
                    }),
                    itunes_ext: Some(rss::extension::itunes::ITunesItemExtension {
                        image: Some(make_url(raiplaysound_url, &card.image)),
                        duration: Some(card.audio.duration),
                        episode: card.episode,
                        season: card.season,
                        ..Default::default()
                    }),

                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }
    }
}

async fn proxy_rss(req: tide::Request<State>) -> tide::Result<tide::Response> {
    let program_name = req.param("program")?;

    let mut url = req.state().raiplaysound_url.clone();
    url.set_path(&format!("programmi/{}.json", program_name));
    let mut program = surf::get(url).await?;

    if program.status() != 200 {
        return Ok(tide::Response::new(404));
    }

    let body_json: RaiPlayProgram = program.body_json().await?;

    Ok(tide::Response::builder(200)
        .body(tide::Body::from_string(
            body_json
                .into_rss(&req.state().raiplaysound_url, &req.state().public_url)
                .to_string(),
        ))
        .content_type("application/xml")
        .build())
}

#[derive(Debug, Clone, serde::Deserialize)]
struct AudioQueryParams {
    pub url: String,
}

const MAX_REDIRECTS: u8 = 3;

// Possible issue: this basically acts as a universal proxy.
// It's kind of hard to vet URLs that pass through this because, from my testing, there's a LOT of domains
// the podcasts could be hosted on.
async fn resolve_audio(url: surf::Url) -> tide::Result<tide::Response> {
    let mut current_url = url;

    // Follow 302 redirects until we hit a 200, but guard against too many redirects
    for _ in 0..MAX_REDIRECTS {
        let mut resp = surf::get(&current_url).send().await?;

        match resp.status() {
            surf::StatusCode::Ok => {
                let mut resp_out = tide::Response::builder(200);

                for name in resp.header_names() {
                    match name.as_str() {
                        "host" | "origin" | "src-fetch-mode" | "src-fetch-site" => {}
                        _ => {
                            resp_out = resp_out.header(name, resp.header(name).unwrap());
                        }
                    }
                }

                resp_out = resp_out.body(resp.take_body());

                return Ok(resp_out.build());
            }
            surf::StatusCode::Found => match resp.header("Location") {
                Some(v) => {
                    current_url = surf::Url::parse(v.last().as_str())?;
                }
                None => {
                    return Ok(tide::Response::builder(500)
                        .body(tide::Body::from_string(
                            "received 302 but no Location header found".to_string(),
                        ))
                        .build())
                }
            },
            code => {
                return Ok(tide::Response::builder(code)
                    .body(tide::Body::from_string(
                        "received unexpected status code".to_string(),
                    ))
                    .build())
            }
        }
    }

    Ok(tide::Response::builder(500)
        .body(tide::Body::from_string("too many redirects!".to_string()))
        .build())
}

async fn proxy_audio(req: tide::Request<State>) -> tide::Result<tide::Response> {
    let query_params: AudioQueryParams = req.query()?;

    let url = match surf::Url::parse(&query_params.url) {
        Ok(url) => url,
        Err(e) => {
            return Ok(tide::Response::builder(500)
                .body(tide::Body::from_string(format!("{e}")))
                .content_type("text/plain")
                .build())
        }
    };

    resolve_audio(url).await
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    let opts = CliOptions::parse();

    tide::log::start();

    let mut app = tide::with_state(State {
        raiplaysound_url: opts.raiplaysound_url,
        public_url: opts.public_url,
    });

    app.at("/programmi/:program").get(proxy_rss);
    app.at("/audio").get(proxy_audio);

    app.listen(opts.bind_address).await?;
    Ok(())
}
