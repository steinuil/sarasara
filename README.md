# sarasara

A proxy that converts the [RaiPlay sound](https://www.raiplaysound.it/) `/programmi/:name.json` API into an RSS feed to be used by podcast applications.

Un proxy che converte l'API `/programmi/:name.json` di [RaiPlay sound](https://www.raiplaysound.it/) in un feed RSS che si può seguire sulle app per sentire i podcast.

## Utilizzo

1. Prendi l'url della pagina del podcast che vuoi ascoltare sul sito RaiPlay Sound. Dovrebbe assomigliare a questo, con il dominio, `/programmi/` e il nome del programma senza niente in coda.

  ```
  https://www.raiplaysound.it/programmi/fantozzidipaolovillaggio
          ^-----dominio-----^           ^--nome del programma--^
  ```

2. Sostituisci `https://www.raiplaysound.it/` con `https://kirarin.hootr.club/sarasara/` (il mio server, non scassatelo per favore) in questo modo:

  ```
  https://kirarin.hootr.club/sarasara/programmi/fantozzidipaolovillaggio
  ```

3. Inserisci quest'ultimo URL nell'app che usi per sentire i podcast per seguirlo.

Gli audiolibri (quelli con `/audiolibri/` nell'url) al momento non sono supportati, se li volete fate un fischio.

Le radio non funzionano e non funzioneranno (come i centri in Albania) perché sono fuori dall'ambito dei podcast.

## Come funziona

`sarasara` espone due endpoint: `/programmi/{program_name}` e `/audio`.

`/programmi/{program_name}` va a prendersi il `.json` corrispondente al nome del programma su Rai Play e lo converte in RSS.

`/audio` è un semplice proxy. Purtroppo è necessario perché i server su cui sono hostati gli audio del podcast settano degli header che dicono ai browser di rifiutare la richiesta, e perché fanno un check sullo User-Agent che blocca client che non sono browser (tipo le app dei podcast). Il proxy toglie questi header e accetta qualunque User-Agent.

L'endpoint prende due parametri nella query: l'url da fare da proxy, e la firma dell'URL. La firma viene generata quando viene creato l'RSS e fa in modo che il proxy possa solo essere utilizzato per URL che vengono dall'RSS.

## Setup sul vostro server

Compila con `cargo run` e piazza sul server. Se eseguito con `--help` mostra tutti i parametri che possono essere impostati.

Sono richiesti questi parametri:

- `SARASARA_PROXY_SECRET` (variabile d'ambiente), che viene usata come chiave per firmare gli URL audio.
- `--public-url`, che diventa la base degli URL audio.
