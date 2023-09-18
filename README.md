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

Le radio non funzionano e non funzioneranno perché sono fuori dall'ambito dei podcast.
