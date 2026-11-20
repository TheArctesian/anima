//! Real `docker run` commands from the documents of published images: each becomes a
//! Compose file with one service that passes the syntax check.

use super::{converted, keys, tree};

/// A command, the name of its service, and the keys of the service in the file.
const COMMANDS: [(&str, &str, &[&str]); 24] = [
    ("docker run hello-world", "hello-world", &["image"]),
    (
        "docker run -it ubuntu bash",
        "ubuntu",
        &["image", "command", "stdin_open", "tty"],
    ),
    (
        "docker run --rm -it alpine:3.20 sh -c 'echo hi'",
        "alpine",
        &["image", "command", "stdin_open", "tty"],
    ),
    (
        "docker run -d -p 6379:6379 --name redis redis:7-alpine redis-server --appendonly yes",
        "redis",
        &["image", "container_name", "command", "ports"],
    ),
    (
        "docker run -d --name mysql -e MYSQL_ROOT_PASSWORD=secret -e MYSQL_DATABASE=app -v mysql_data:/var/lib/mysql -p 3306:3306 mysql:8.4",
        "mysql",
        &["image", "container_name", "ports", "environment", "volumes"],
    ),
    (
        "docker run -d --name mongo -p 27017:27017 -v /srv/mongo:/data/db mongo:7",
        "mongo",
        &["image", "container_name", "ports", "volumes"],
    ),
    (
        "docker run -d --restart=always -p 9000:9000 -p 9443:9443 -v /var/run/docker.sock:/var/run/docker.sock -v portainer_data:/data portainer/portainer-ce:latest",
        "portainer-ce",
        &["image", "restart", "ports", "volumes"],
    ),
    (
        "docker run -d --name=grafana -p 3000:3000 --user 472 -v grafana-storage:/var/lib/grafana grafana/grafana-oss",
        "grafana",
        &["image", "container_name", "ports", "volumes", "user"],
    ),
    (
        "docker run -d --name prometheus -p 9090:9090 -v ./prometheus.yml:/etc/prometheus/prometheus.yml:ro prom/prometheus --config.file=/etc/prometheus/prometheus.yml",
        "prometheus",
        &["image", "container_name", "command", "ports", "volumes"],
    ),
    (
        "docker run -d --name traefik -p 80:80 -p 443:443 -l traefik.enable=true --label 'traefik.http.routers.api.rule=Host(`traefik.example.com`)' traefik:v3.1 --providers.docker",
        "traefik",
        &["image", "container_name", "command", "ports", "labels"],
    ),
    (
        "docker run -d --name=jellyfin --net=host --device /dev/dri:/dev/dri -v /srv/media:/media:ro --restart unless-stopped jellyfin/jellyfin",
        "jellyfin",
        &[
            "image",
            "container_name",
            "restart",
            "volumes",
            "network_mode",
            "devices",
        ],
    ),
    (
        "docker run -d --name pihole -p 53:53/tcp -p 53:53/udp -e TZ=America/Los_Angeles --cap-add NET_ADMIN --dns 127.0.0.1 --dns 1.1.1.1 --hostname pi.hole pihole/pihole:latest",
        "pihole",
        &[
            "image",
            "container_name",
            "ports",
            "environment",
            "cap_add",
            "dns",
            "hostname",
        ],
    ),
    (
        "docker run -d --name ha --privileged --network host -e TZ=UTC -v /srv/ha:/config ghcr.io/home-assistant/home-assistant:stable",
        "ha",
        &[
            "image",
            "container_name",
            "environment",
            "volumes",
            "network_mode",
            "privileged",
        ],
    ),
    (
        "docker run -d --name watchtower -v /var/run/docker.sock:/var/run/docker.sock containrrr/watchtower --interval 3600",
        "watchtower",
        &["image", "container_name", "command", "volumes"],
    ),
    (
        "docker run --name vault --cap-add=IPC_LOCK -e 'VAULT_LOCAL_CONFIG={\"storage\": {\"file\": {\"path\": \"/vault/file\"}}}' -p 8200:8200 hashicorp/vault server",
        "vault",
        &[
            "image",
            "container_name",
            "command",
            "ports",
            "environment",
            "cap_add",
        ],
    ),
    (
        "docker run -d --name es -p 9200:9200 -e discovery.type=single-node --ulimit nofile=65535:65535 --memory 2g --cpus 1.5 docker.elastic.co/elasticsearch/elasticsearch:8.15.0",
        "es",
        &[
            "image",
            "container_name",
            "ports",
            "environment",
            "ulimits",
            "mem_limit",
            "cpus",
        ],
    ),
    (
        "docker run -d --name minio -p 9000:9000 -p 9001:9001 --env-file ./minio.env -v minio:/data quay.io/minio/minio server /data --console-address ':9001'",
        "minio",
        &[
            "image",
            "container_name",
            "command",
            "ports",
            "env_file",
            "volumes",
        ],
    ),
    (
        "docker run -d --name app --read-only --tmpfs /tmp --security-opt no-new-privileges --cap-drop ALL --init registry.example.com:5000/team/app@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "app",
        &[
            "image",
            "container_name",
            "read_only",
            "tmpfs",
            "security_opt",
            "cap_drop",
            "init",
        ],
    ),
    (
        "docker run -d --name gitea -p 3000:3000 -p 2222:22 -v gitea:/data -v /etc/timezone:/etc/timezone:ro --add-host host.docker.internal:host-gateway gitea/gitea:1.22",
        "gitea",
        &["image", "container_name", "ports", "volumes", "extra_hosts"],
    ),
    (
        "docker run -dit --name box -w /work -h box --entrypoint /bin/sh --stop-signal SIGINT --stop-timeout 30 busybox",
        "box",
        &[
            "image",
            "container_name",
            "entrypoint",
            "stdin_open",
            "tty",
            "working_dir",
            "hostname",
            "stop_signal",
            "stop_grace_period",
        ],
    ),
    (
        "docker run -d --name ollama --gpus all -v ollama:/root/.ollama -p 11434:11434 --shm-size 1g --platform linux/amd64 --pull always ollama/ollama",
        "ollama",
        &[
            "image",
            "container_name",
            "ports",
            "volumes",
            "gpus",
            "shm_size",
            "platform",
            "pull_policy",
        ],
    ),
    (
        "docker run -d --name nc --link db:db --volumes-from data --expose 9000 --group-add audio --pid host --ipc host --sysctl net.core.somaxconn=1024 nextcloud:29",
        "nc",
        &[
            "image",
            "container_name",
            "links",
            "volumes_from",
            "expose",
            "group_add",
            "pid",
            "ipc",
            "sysctls",
        ],
    ),
    (
        "docker run -d --name log --log-driver json-file --log-opt max-size=10m --log-opt max-file=3 --health-cmd 'wget -qO- localhost' --health-interval 30s --health-timeout 5s --health-start-period 1m nginx",
        "log",
        &["image", "container_name", "logging", "healthcheck"],
    ),
    (
        "/usr/bin/docker container run --name=web2 --no-healthcheck -P -q nginx:1.27-alpine",
        "web2",
        &["image", "container_name", "healthcheck"],
    ),
];

#[test]
fn host_022_each_real_command_gives_a_file_that_passes_the_check() {
    for (command, service, expected) in COMMANDS {
        let done = converted(command);
        assert_eq!(done.service, service, "{command}");
        let file = tree(&done.compose_file);
        assert_eq!(keys(&file, &["services"]), [service], "{command}");
        assert_eq!(keys(&file, &["services", service]), expected, "{command}");
    }
}
