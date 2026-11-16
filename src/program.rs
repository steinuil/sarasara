use serde::{Deserialize as _, Deserializer};
use url::Url;

use crate::{Params, signature};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Genre {
    pub name: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PodcastInfo {
    pub title: String,
    pub description: String,
    pub image: String,
    pub editor: String,
    pub weblink: String,
    pub genres: Vec<Genre>,
    pub subgenres: Vec<Genre>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct TrackInfo {
    #[serde(deserialize_with = "deserialize_date")]
    pub date: jiff::civil::Date,
    pub page_url: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Audio {
    pub url: String,
    pub duration: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Card {
    pub uniquename: String,
    pub toptitle: String,
    pub description: String,
    pub audio: Audio,
    pub image: String,
    pub track_info: TrackInfo,
    pub episode: Option<String>,
    pub season: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Block {
    pub cards: Vec<Card>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RaiPlayProgram {
    pub podcast_info: PodcastInfo,
    pub block: Block,
}

impl RaiPlayProgram {
    pub fn into_rss(self, params: &Params) -> rss::Channel {
        rss::Channel {
            title: self.podcast_info.title.clone(),
            link: make_url(&params.raiplaysound_url, &self.podcast_info.weblink),
            description: self.podcast_info.description.clone(),
            managing_editor: Some(self.podcast_info.editor.clone()),
            categories: self
                .podcast_info
                .genres
                .into_iter()
                .chain(self.podcast_info.subgenres)
                .map(|genre| rss::Category {
                    name: genre.name,
                    domain: None,
                })
                .collect(),
            generator: Some("sarasara https://github.com/steinuil/sarasara".to_string()),
            image: Some(rss::Image {
                url: make_url(&params.raiplaysound_url, &self.podcast_info.image),
                title: self.podcast_info.title,
                description: Some(self.podcast_info.description.clone()),
                link: make_url(&params.raiplaysound_url, &self.podcast_info.weblink),
                ..Default::default()
            }),
            items: self
                .block
                .cards
                .into_iter()
                .map(|card| rss::Item {
                    title: Some(card.toptitle),
                    link: Some(make_url(
                        &params.raiplaysound_url,
                        &card.track_info.page_url,
                    )),
                    description: Some(card.description),
                    pub_date: Some(
                        jiff::fmt::rfc2822::to_string(
                            &card
                                .track_info
                                .date
                                .at(0, 0, 0, 0)
                                .in_tz("Europe/Rome")
                                .expect("Europe/Rome to be a valid timezone"),
                        )
                        .expect("RFC 2822 formatting"),
                    ),
                    enclosure: Some(rss::Enclosure {
                        url: make_audio_url(
                            &params.public_url,
                            &params.proxy_secret,
                            &card.audio.url,
                        ),
                        mime_type: "audio/mpeg".to_string(),
                        length: "".to_string(),
                    }),
                    guid: Some(rss::Guid {
                        value: card.uniquename,
                        permalink: false,
                    }),
                    itunes_ext: Some(rss::extension::itunes::ITunesItemExtension {
                        image: Some(make_url(&params.raiplaysound_url, &card.image)),
                        duration: Some(card.audio.duration),
                        episode: card.episode,
                        season: card.season,
                        ..Default::default()
                    }),

                    ..Default::default()
                })
                .collect(),

            itunes_ext: Some(rss::extension::itunes::ITunesChannelExtension {
                author: Some(self.podcast_info.editor),
                summary: Some(self.podcast_info.description),
                image: Some(make_url(&params.raiplaysound_url, &self.podcast_info.image)),
                ..Default::default()
            }),

            ..Default::default()
        }
    }
}

fn deserialize_date<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<jiff::civil::Date, D::Error> {
    let value = String::deserialize(deserializer)?;
    jiff::civil::Date::strptime("%Y-%m-%d", &value).map_err(serde::de::Error::custom)
}

fn make_url(base_url: &Url, path: &str) -> String {
    let mut url = base_url.clone();
    url.set_path(path);
    url.to_string()
}

fn make_audio_url(base: &Url, secret: &signature::Secret, url_param: &str) -> String {
    let mut url = base.clone();

    let path = if url.path() == "/" {
        "/audio".to_string()
    } else {
        format!("{}/audio", url.path())
    };
    url.set_path(&path);

    // Sign the URL so that this cannot be used as a general public proxy of sorts
    let sig = signature::sign(secret, url_param);

    url.query_pairs_mut()
        .append_pair("url", url_param)
        .append_pair("sig", &sig);

    dbg!(url.query());

    url.to_string()
}
