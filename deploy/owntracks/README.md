# A quick OwnTracks server

| Item | Value |
| --- | --- |
| Document | How to start an OwnTracks Recorder for one person, and how its samples reach Anima |
| Owner | [extensions-plan.md](../../docs/plans/extensions-plan.md) Section 5.1, E0-5 task T6 (R-145) |
| Date | 2026-11-18 |
| Status | Built. Not started on a real computer by an agent: its check is an entry of [need-human-testing.md](../../docs/need-human-testing.md), written with E0-5 task T5. |

This document is a procedure. It adds no requirement.

## Do you need this server?

No, if you only want your location in Anima. The OwnTracks app can post to the Anima hub with
no server between them: make a token with `anima location token add phone`, then do step 4 with
the address `https://<your hub>/locations/v1/owntracks` and the token as the password.

Use this server if you also want the map and the history of OwnTracks itself. It keeps each
sample in the Recorder and sends a copy to Anima.

## What runs

| Service | Image | Purpose |
| --- | --- | --- |
| `recorder` | `owntracks/recorder` | Stores the samples and shows the map. No MQTT. No published port. |
| `proxy` | `nginx` | Asks for the password, gives each post to the Recorder, and sends a copy to Anima. Published on `127.0.0.1:8083`. |

The Recorder has no password and no TLS of its own, so it is only on the inner network. Anima
copies no code of OwnTracks; the two images are used as they are (the Recorder is GPL-2.0-or-later).

## Steps

You need a computer with Docker that is always on. These steps change only that computer.

1. Copy this folder to the computer and go into it.
2. Run `./setup.sh`. It prints the user name and the password one time. Write them down.
   To send a copy to Anima, make an ingest token on a computer with the `anima` command
   (`anima location token add owntracks`), put it in a file, and run `./setup.sh < that-file`.
   Then delete the file.
3. Run `ANIMA_OWNTRACKS_URL=https://<your hub>/locations/v1/owntracks docker compose up -d`.
   Leave `ANIMA_OWNTRACKS_URL` out if no copy goes to Anima. Expected result:
   `docker compose ps` shows `recorder` and `proxy` as running.
4. Give the server an address with TLS. The phone must not send the password with no TLS.
   Use the tunnel or the reverse proxy that you have (for example a Cloudflare tunnel to
   `http://127.0.0.1:8083`), or put the computer and the phone on the same VPN and set
   `OWNTRACKS_ADDRESS` to the VPN address of the computer.
5. In the OwnTracks app: Settings > Connection > Mode `HTTP`; URL `https://<address>/pub`;
   Authentication on, with the user name and the password of step 2; Device ID `phone`;
   Tracker ID two characters of your choice. Then send a location by hand (the upload button).
   Expected result: the map at `https://<address>/` asks for the password and shows the point.
6. If a copy goes to Anima: `anima location list` shows the same sample, with the provider
   `owntracks`.

## Stop and remove

`docker compose down` stops the two services and keeps the samples. `docker compose down -v`
also deletes the samples of the Recorder. It deletes nothing in Anima.

## Faults

| What you see | Cause | Do this |
| --- | --- | --- |
| The app shows HTTP 401 | Wrong user name or password | Delete `secrets/htpasswd`, run `./setup.sh` again, `docker compose restart proxy` |
| The map shows the point and Anima does not | No token, a wrong hub address, or a revoked token | `docker compose logs proxy`; make a new token and run `./setup.sh < file`, then `docker compose restart proxy` |
| `proxy` does not start | `secrets/` is missing | Run `./setup.sh` first |
