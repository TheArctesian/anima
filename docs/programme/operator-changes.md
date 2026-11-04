# Operator changes applied by the programme orchestrator (R-025)

Each row: time, system, change, how it was checked, how to undo it.

| Time | System | Change | Check | Undo |
| --- | --- | --- | --- | --- |
| 2026-11-04 09:18Z | local Mac | Created SSH key `~/.ssh/anima_ops_ed25519` (no passphrase, comment `anima-ops-20261104`, SHA256:E9HW7Ow77gBDPQ9EokBqi0yiSUCRkglYXcQaxiEwQ5c) for this run | `ssh-keygen -lf` | `rm ~/.ssh/anima_ops_ed25519*` |
| 2026-11-04 09:19Z | Hostinger API | Registered that public key (id 618466) and asked to attach it to VPS 1656033; the API returned action id 0 and SSH still refuses it, so it is not on the server | `ssh -i … root@31.220.53.226` denied | `hostinger vps public-keys delete 618466` |
| 2026-11-04 09:30Z | Cloudflare zone `anima.ing` | WP-03 runbook step 1: A `hs.anima.ing` → 31.220.53.226, not proxied, TTL 300 (record fa990a48e5ddbc9df4261bcdb6d52152); CAA `anima.ing` `0 issue "letsencrypt.org"` (record f4efffb67cbe83149f9951bb2a0d834e). Applied through the Cloudflare API connection; no token file was made | `dig +short hs.anima.ing @1.1.1.1`, `dig +short CAA anima.ing @1.1.1.1` | `DELETE /zones/f020a469ff43990b3070a300ea1634c1/dns_records/<id>` for each |
