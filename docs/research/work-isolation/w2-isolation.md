# Record: the isolation of a job, with a prototype on one Mac (task W2)

Version: 1. Date: 2026-11-18.
Status: Partly measured. One Mac is measured. No Linux computer is measured, and no second user of macOS was made: each needs the operator. The design in Section 4 is a proposal. The security reviewer and the operator did not accept it.

This document is a record (work plan, Section 9; PLN-010). It adds no requirement. Its Section 4 is the input for the owner of the requirements, [projects-and-builds-plan.md](../../plans/projects-and-builds-plan.md) (PRO-022 to PRO-024, PRO-029), and for the contract [work-route.md](../../../contracts/work-route.md).

## 1. Purpose

Task W2 asks for a design with measured results for four subjects: the container runtime on Linux, a workflow that needs Docker, the build user on a Mac, and a signing identity for one job. The code of a job is not trusted (PRO-029): it can be the code of a pull request from a fork (fact F20 of the plan).

## 2. Method

- The computer: a Mac with Apple Silicon, 18 processor cores, 48 GiB of memory, macOS 27.3, Docker Desktop 29.9.3 (a Linux virtual machine with the kernel 7.0.14-linuxkit).
- The scripts are in this folder: `seatbelt.sh`, `job-keychain.sh`, `container-job.sh`, `docker-in-job.sh`. They are throwaway code of a prototype, not code of the product; nothing of the product calls them. Their logs are in [results/](results/). In the logs, the home folder of the user is written as `<home>`. The measurements stopped before their end on 2026-11-18, when a release build needed the computer: Section 3.4 says what that leaves open.
- The computer did other work at the same time: the load average was between 18 and 45. Each time below was taken under that load. Read a time as an upper value, not as the time of a quiet machine.
- No administrator right was used. Thus no user was made, no system unit was installed and no system setting was changed. No certificate of Apple and no key of the operator was read: the signing test made its own throwaway certificate.

## 3. Results

### 3.1 A native job on a Mac with a sandbox profile and no second user (`seatbelt.sh`)

The job ran with `sandbox-exec` and a profile that permits each thing by default, denies the reading of the home folder and of a stand-in for the daemon token, and denies each write outside the job folder.

| Attempt of the job | Result |
| --- | --- |
| List the home folder, list `~/.ssh`, read the file of the login keychain, read the stand-in daemon token | Denied |
| Write to the home folder, to `/tmp`, to the stand-in folder of the daemon | Denied |
| Read and write in the job folder; start a program from it | Permitted |
| `git` with `HOME` set to the job folder; `cc` and `swiftc` of a small program; an ad hoc `codesign` | Worked. `git` with the usual `HOME` failed, because it reads `~/.gitconfig` |
| `curl` to a public address | Permitted |
| List the processes of the computer (`ps`) | Denied |
| `security find-identity` | Ran. It found 0 identities; outside the sandbox the same command found 6 |
| `security find-generic-password` for an item that does not exist | **The keychain service answered** ("could not be found"). Thus the job can ask the keychain service of the user. It still answered with the service names of the keychain denied in the profile, and with each service lookup denied. The test read no real item on purpose, so it does not show if a real item can be read |
| Connect to the Docker socket of the computer | **Worked.** The file of the socket is in the home folder, but a connection is not a file read |
| Start overhead of `sandbox-exec`, `/usr/bin/true`, 20 runs | Median 8.6 ms with it, 2.5 ms without it |

`man sandbox-exec` marks the program "DEPRECATED".

What this shows: a profile on the user of the operator hides files. It does not hide the services of that user. The Docker socket of the computer stayed usable, and the keychain service still answered. A job with the Docker socket controls the computer. A profile alone thus does not meet PRO-022.

### 3.2 A signing identity for one job (`job-keychain.sh`)

| Step | Time | Result |
| --- | --- | --- |
| Make a keychain file in a temporary folder, with a random password | 12 ms | The search list of the user did not change |
| Import a key and a certificate for `codesign` only; set the partition list | 31 ms, 15 ms, 14 ms | No window asked a question; no command waited |
| `codesign --keychain <file> -s <name>` | 38 ms | Worked with the keychain **not** in the search list |
| `codesign --verify` | 15 ms | "valid on disk" |
| `security delete-keychain`, remove the folder | 10 ms | The file is gone. `codesign` with that name then fails: "no identity found" |
| The search list and the default keychain, before and after | — | The same |

A limit of the tool: the password of the keychain is an argument of `security`, so a process of the same user can read it from the list of processes while the command runs.

Not shown: a Developer ID identity and the notarization from such a keychain. The test used a certificate that it made itself.

### 3.3 A container job (`container-job.sh`)

Each job: a new container of `alpine:3` with `--rm --cap-drop ALL --security-opt no-new-privileges --pids-limit 512 --memory 2g --read-only`, a temporary file system as its work folder, a user that is not root, one new empty folder of the host, and no Docker socket.

| Measure | Result |
| --- | --- |
| From `docker run` to the end of the first command, 10 runs | Median 349 ms (191 to 594) |
| Memory of an idle container | 2.5 MiB |
| Inside: the user, the capabilities, new privileges | uid 1000; no capability; `NoNewPrivs: 1` |
| Inside: a Docker socket; a write to `/` | No socket; "Read-only file system" |
| Host paths inside | The one job folder, and three files that Docker gives (`hostname`, `hosts`, `resolv.conf`) |
| After the end, and after `docker kill` in the middle of a `sleep` | The container and the job folder are gone; the removal took 184 ms |

Not shown: this is Docker Desktop, where each container is already inside a virtual machine. A Linux computer has no such second wall. The network of the job was not limited in this test.

### 3.4 A job that needs Docker (`docker-in-job.sh`)

A second Docker service ran beside the job, on a network of its own. The job reached it over TLS and had no socket of the host.

This measurement is not complete. The script ran two times to its end, and the session read the two results. A third run, with more sets of flags, wrote over the log and was stopped before its end, because a release build needed the computer. The log in [results/](results/) is that stopped run. The values below are those of the first two runs; no log on disk shows them. Run the script again on a quiet computer before these values are used.

| Image of the service | Flags that were tried | Result |
| --- | --- | --- |
| `docker:dind` | None; no seccomp and no AppArmor profile; plus `SYS_ADMIN` and `NET_ADMIN`; `/dev/net/tun` | Did not start: "mount: permission denied" |
| `docker:dind` | `--privileged` | Started in 2.2 s (first run) and 0.9 s (second run). Idle: 28 MiB and 31 MiB, 15 or 16 processes. The job ran `hello-world` in 1.9 s and 5.6 s, with the pull. The inner containers were not in the list of the host. After the removal of the service, no inner process stayed |
| `docker:dind-rootless` | First run: each of four sets, `--privileged` included | Did not start |
| `docker:dind-rootless` | Second run: no seccomp profile, not privileged | The service started in 1.0 to 1.3 s; idle 60 MiB. The `hello-world` of the job failed: "failed to create task for container" |

The size of the two images: 106 MB and 122 MB to pull, 421 MB and 470 MB on disk.

What this shows, as far as it goes: on this computer the only service in which the job could run a container needed `--privileged`. A privileged container can leave its container. Here it would reach only the virtual machine of Docker Desktop. On a Linux computer it would reach the computer.

## 4. The design that these results support (a proposal)

### 4.1 Linux: the container runtime

- One job, one new container, made by a runtime that has no root right: Podman without root, or Docker in its rootless mode, under a system user that exists only for jobs. That user is not the user of `anima-daemon`, has no login and owns nothing else.
- The flags of Section 3.3 are the floor: no capability, no new privileges, a read-only root, a work folder with a size limit, limits for processes, memory and time, a user that is not root inside.
- The network of a job: the public internet only. No loopback address of the computer, no interface of the link or of the VPN, no private address range, and not the origin of the hub. This closes the path of finding SF-004 for a container job and gives a job no way to the hub (contract K10, Section 6.3).
- The job folder is the only path of the computer in the job. It is made new and deleted at each end, also at the start of the daemon after a stop.
- For a repository that is public and takes pull requests from forks, a container on a shared kernel is one wall. The proposal for the security reviewer: such a repository gets a small virtual machine for each job, not a container. Not measured.

To measure on the Linux computer: the start time and the memory of a rootless container; that the limits hold (a job that fills the memory, the disk, the process table); that the network rule holds; what stays after `kill -9` of the daemon in the middle of a job.

### 4.2 A workflow that needs Docker

- The Docker socket of the computer never goes into a job (the open point of the plan). The contract has no member that could ask for it.
- First version: a job has no Docker. A workflow with a `container:` job or a Docker action fails with a sentence that says so. The set-up says it before the press, and the text to copy tells the coding agent to stop (PRO-058).
- Later: a job that needs Docker gets a virtual machine of its own, with its own Docker service inside. The result of Section 3.4 is the reason: the service in a container needed `--privileged`.
- To measure on the Linux computer: rootless Docker inside a rootless container with user namespaces, and one small virtual machine for each job (start time, memory, disk). If the first works with no privileged flag on Linux, it is the cheaper way.

### 4.3 The Mac: the build user

- A profile alone is not sufficient (Section 3.1). The job runs as a second user of macOS that is made for builds (PRO-022): a standard user, not an administrator, with a home folder of its own and an empty keychain. Then the keychain service and the Docker socket that the job reaches are those of the build user, not those of the operator.
- The profile of Section 3.1 stays as a second layer on the build user: it denies the home folder of the operator by path, and it limits the writes to the job folder. The file modes alone are not a sure wall: on this Mac the home folder has the mode `drwxr-x---`, so a user in its group can read it, and the usual group of a standard user of macOS is that group. The build user must thus not be in the group of the operator; to check with an administrator.
- An administrator makes the build user one time, and installs one small program that starts a job as that user. That program is owned by root, takes no command from the daemon, and starts only the fixed job starter with the folder of one job. This is the step that PRO-024 gives to the operator.
- At each end the machine ends each process of the build user, and deletes the job folder, the temporary folder of that user and the keychain of the job.
- One Mac has one build user, so it runs one native job at a time.
- For code of a fork on a public repository, the stronger wall is a virtual machine of macOS for each job. Not measured.

To measure with an administrator on a Mac: that the build user cannot read the home folder and the keychain of the operator, with and without the profile; that no process of the build user stays after a stop; the time to make and to delete the job folder with a real checkout; that Xcode builds as that user.

### 4.4 A signing identity for one job

Two different needs have two different answers.

- The release of Anima. No job gets the Developer ID identity. A warm runner takes the next job that names its labels, and the hub does not select that job (contract K10, Section 6.2), so an identity inside a runner job could reach a job of a fork. The proposal: the job builds a bundle with no signature and gives it to the hub that checks it (OD-W31, choice A). The signature of the Mac is then a step of its own, outside each job: a fixed recipe of the daemon on the Mac that has the identity in its keychain (R-116, BLD-083 to BLD-085). It signs, notarizes and staples the bytes whose SHA-256 the hub gives, and it runs no code of the repository. R-116 does not change: the identity stays in the keychain of that Mac. A second way, for a later decision: the hub that signs also signs the Mac bundle. That moves the identity to a server and changes R-116.
- The project of a different repository, whose workflow signs by itself. The owner of that repository gives the identity as a secret of GitHub, and the workflow imports it. The keychain of Section 3.2 is the form that the machine supports: a file in the job folder, never in the search list, deleted with the job. Anima gives such a job no identity (PRO-030).

The contract K10 thus has no purpose `signing_identity` in its version 0.2. If the security reviewer and the operator accept this section, it needs none.

## 5. What is not shown, and what it needs

| Not shown | Needs |
| --- | --- |
| Each measurement of Section 4.1 and 4.2 on Linux | One Linux computer that the operator names, on which an agent can install a container runtime without root and run the scripts |
| The build user (Section 4.3) | An administrator on a Mac, one time: make the user and the starter program. Then an agent can measure |
| A virtual machine for each job, on Linux and on a Mac | The same two computers |
| A Developer ID signature and a notarization by the recipe of Section 4.4 | The operator: the identity and the notary profile are in his keychain. An agent does not read them |
| The times on a quiet machine, and a complete result of Section 3.4 | A run of the four scripts when the computer does no other work. The measurements stopped on 2026-11-18 for a release build |
| A list of the build tools of this Mac with their versions | Not made: the measurements stopped before it |
| The network limits of a job | Linux; on a Mac, a rule of the firewall needs an administrator |

## 6. For the acceptance

The security reviewer and the operator accept the design, not this record. The sentence of the operator:

> I accept the isolation design of task W2 as Section 4 of `docs/research/work-isolation/w2-isolation.md` proposes it, with the measurements of Section 5 still to do on my Linux computer and with an administrator on my Mac.

Before that sentence, the requirements that Section 4 would add (the network rule of a job, no Docker in the first version, the sign recipe) go into [projects-and-builds-plan.md](../../plans/projects-and-builds-plan.md) with identifiers, as a change of that plan.
