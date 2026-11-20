# Record: the applications program on a Mac and with a rootless Docker (task H30)

Version: 1. Date: 2026-11-19.
Status: Partly measured. One Mac with Docker Desktop is measured, read-only. Not measured: a Linux computer with the system Docker, a rootless Docker, and a Mac with Colima, OrbStack or Rancher Desktop; Section 6 lists the steps for each. The design in Section 5 is options for the operator (OD-HOST-27 of [hosted-applications-plan.md](../../plans/hosted-applications-plan.md)). No reviewer and no ruling accepted it.

This document is a record (work plan, Section 9; PLN-010). It adds no requirement. It is the input of task H30 for the owner of the requirements, [hosted-applications-plan.md](../../plans/hosted-applications-plan.md), and for the text of the container work types of contract K10 ([hosted-applications-stage-2.md](../../proposals/hosted-applications-stage-2.md), Section 4).

## 1. Purpose

R-226 (operator, 2026-11-19) says: a small program with the rights of the administrator of a machine is the only program of Anima that talks to Docker, and the daemon has no right to use Docker. The second read of the contract text found a fault for a Mac (SF-035 of [security-findings.md](../../security-findings.md)): there the Docker program and its engine belong to the user, so a program that runs as the administrator and starts them can be made to run code of the user as the administrator.

R-237 (operator, 2026-11-19) says: the contract text does not become version 0.4 until this design exists for a Mac and he has approved it, and it must cover a rootless Docker on Linux.

## 2. Method

Each value of Section 3 was read on 2026-11-19 on the build machine with commands that only read: `ls`, `stat`, `readlink`, `docker context ls`, `docker info`, `docker version`, `ps`, and one read of the settings file of Docker Desktop. Nothing was installed, no setting was changed, and no container was made for this record. The machine: macOS 27.3 on Apple silicon, Docker Desktop with the engine 29.9.3 and Compose 5.7.0, one account, which is an administrator account.

What a read-only look cannot show is in Section 6.

## 3. What was measured on a Mac with Docker Desktop

| Subject | Measured |
| --- | --- |
| The `docker` command | `/usr/local/bin/docker` is a link that root owns. It points into `/Applications/Docker.app/Contents/Resources/bin/`. That folder and the program in it belong to the user of the Mac, who can write them. |
| The Compose plug-in | `~/.docker/cli-plugins/docker-compose` is a link in the home folder of the user. It points into `Docker.app`, to a file that the user owns and can write. No plug-in folder of the system exists (`/usr/local/lib/docker/cli-plugins` is absent). |
| The socket of the engine | `~/.docker/run/docker.sock`, owned by the user. `/var/run/docker.sock` is a link, owned by root, to that socket. A helper of Docker that runs as root (`/Library/PrivilegedHelperTools/com.docker.socket`, a launch daemon with the user name as its argument) makes the link. |
| The current context | `desktop-linux`, at the socket in the home folder. The context `default` names `/var/run/docker.sock`, which is the same socket. |
| Who runs the engine | Each process of Docker Desktop runs as the user (`Docker`, `com.docker.backend`, `com.docker.build`). No process of Docker runs as root, apart from the helper that makes the link. |
| Where the engine is | In a Linux virtual machine: `docker info` gives the operating system "Docker Desktop" and a `linuxkit` kernel. Its security options do not name `rootless`: in the virtual machine the engine runs as root. |
| A group | The user needs no group to use Docker. The account is in `admin`; no group `docker` exists. |
| Other runtimes | None on this machine: no Colima, OrbStack, Rancher Desktop, Podman or Lima. |

What follows from the table:

1. **Each program of the user already holds the Docker of the Mac.** The socket belongs to the user. The daemon of Anima on a Mac is a unit of the logged-in user (R-206, R-208). Thus R-226 cannot be made true on such a Mac by any program of Anima: the daemon can open the socket, with or without an applications program.
2. **The Docker right is not the right of the administrator of the Mac.** It is the right over a virtual machine and over the folders that Docker Desktop shares with it. A container that mounts a shared folder reads and writes it as the user. Which folders are shared was not readable from the settings file (Section 6); the documentation of Docker says that the home folders are shared by default (not verified).
3. **A program of Anima that ran as the administrator would add a risk and give no protection.** It would start a program that the user can replace (SF-035), to reach an engine that the user holds in any case.

## 4. The two kinds of engine

The measured Mac shows a second kind of engine beside the one that the contract text assumed.

| Kind | Who owns the engine and its socket | What the Docker right is | Where (measured or not) |
| --- | --- | --- | --- |
| A system engine | Root. A user needs a group or root to use it | The right of the administrator of the machine | Linux with Docker from a package. Not measured for this record |
| An engine of a user | The user | The right of that user over the engine, and over what the engine can reach | Docker Desktop on a Mac: measured. Rootless Docker on Linux, Colima, OrbStack, Rancher Desktop, Docker Desktop on Linux: from their documentation, not measured |

For an engine of a user, the engine of Docker in its rootless mode runs as a user and has its socket in the folder of that user (`$XDG_RUNTIME_DIR/docker.sock`); a container of it cannot become the administrator of the machine by the usual ways, because root in the container is that user outside (from the documentation of Docker; not measured).

## 5. Design options

The question for the operator: what is the applications program on a machine whose engine belongs to a user?

### Option A: the applications program runs as the owner of the engine

- With a system engine it runs as the administrator, as R-226 says. It refuses to start when the `docker` program, the Compose plug-in or a folder above them can be written by a user other than the administrator.
- With an engine of a user it runs as that user, as a second unit beside the daemon. It never runs as the administrator there. It still is the only program of Anima that talks to Docker, and it applies each rule of the contract to each request.
- The settings of the machine ("Manage applications" and the others) stay in a file that only the administrator can write, on each kind of machine. To set one, the owner is asked for the administrator one time. The applications program only reads the file.
- What the rules then protect, on an engine of a user: the path from the hub. A person who controls the hub, an API key and an AI agent get what the rules permit and no more, because their requests reach Docker only through the applications program. What the rules do not protect there: a program of the same user, which can use the engine with no Anima at all. Anima adds no right to such a program; it had the engine before.
- The contract says this in plain words, and the screen that sets "Manage applications" on says which kind of engine the machine has.

For: one design for each machine of the operator. No program of Anima runs as the administrator where that gives nothing. It removes SF-035. It is true to what was measured.
Against: on a Mac, R-226 holds only in part: "the only program of Anima that talks to Docker" holds, "the daemon has no right to use Docker" cannot. The operator must accept that sentence for an engine of a user. A fault of the daemon that a hub can reach then reaches the engine on a Mac.

### Option B: on a Mac, the applications program runs as the administrator with its own copy of the Docker programs

- Anima installs its own `docker` and Compose programs in a folder of the administrator and starts only those. It talks to the socket of the user.

For: it starts no program that the user can write.
Against: the engine and its socket still belong to the user, so a program of the user can stand in for the engine and answer the administrator program with text of its choice; the daemon can still open the socket, so R-226 is no more true than with option A. Anima must then ship and update two programs of Docker for each system. It is more to build for no added limit.

### Option C: no container work on an engine of a user

- Stage 2 works only on machines with a system engine. A Mac and a rootless Docker keep the read-only screens of stage 1.

For: the contract needs no second case.
Against: it is option A of OD-HOST-25, which the operator did not select (R-237): he wants one version that covers each of his machines.

### Recommendation

Option A. It needs one sentence of acceptance from the operator: on a machine whose engine belongs to a user, the daemon can reach Docker, and the rules protect against the hub and not against a program of that user. This changes R-226 for such a machine and needs a ruling.

With option A the open point of the settings (the hub that a daemon is enrolled at cannot be read in a way that the user cannot forge; task H30, fact 2 in the plan) stays open for each kind of engine. It waits for the key of the device (task W7), as R-236 says for the requests.

## 6. What is not measured, and the steps

The record is read-only. Each line is a thing that a person, or a session on that machine, must do.

| Not measured | Steps | Expected |
| --- | --- | --- |
| The folders that Docker Desktop shares with its virtual machine | In Docker Desktop: Settings, Resources, File sharing. Write down the list | The home folders are in the list. If the folder that Anima uses for the files of an application is not below one of them, a bind mount of an application does not work |
| That a container reads a shared folder as the user | `docker run --rm -v "$HOME":/h alpine:3 ls /h` | The names of the home folder. This makes one container and removes it |
| A Linux computer with the system Docker | `stat -c '%U:%G %a %n' /var/run/docker.sock "$(command -v docker)"`; `docker info --format '{{.SecurityOptions}}'`; `ls -la /usr/libexec/docker/cli-plugins /usr/lib/docker/cli-plugins` | The socket is `root:docker` with mode 660; the program and the plug-ins belong to root; the security options do not name `rootless` |
| A rootless Docker on Linux | As a user with rootless Docker: `docker context ls`; `stat -c '%U %n' "$XDG_RUNTIME_DIR/docker.sock"`; `docker info --format '{{.SecurityOptions}}'`; `docker run --rm -v /:/host alpine:3 touch /host/etc/anima-probe` | The socket belongs to the user; the security options name `rootless`; the last command is refused, because root in the container is the user outside |
| A Mac with Colima or OrbStack | `docker context ls`; `stat -f '%Su %N'` on the socket of the current context and on `$(command -v docker)` | The socket and the program belong to the user, as with Docker Desktop |
| That the daemon as a LaunchAgent can open the socket of Docker Desktop | With the daemon of the desktop application installed (task HM-T19): read its log for the tool inventory, which runs `docker --version` | The inventory names the version. This shows the reach of the unit, not a use of the socket |

## 7. What this record changes in the text of the contract

When the operator has decided OD-HOST-27, the text of the container work types (Section 4.1 of the proposal) gets: the two kinds of engine; who the applications program runs as for each; the refusal to start when a program of Docker can be written by a user other than the one it runs as; the sentence of what the rules protect on an engine of a user; and what the screen says. The owner of `contracts/work-route.md` reviews that text.
