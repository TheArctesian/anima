//! Output captured from a Linux 6.x host and a macOS 15 host.

pub const PROC_STAT: &[u8] = b"cpu  4705 356 584 3699 23 23 0 0 0 0\n\
cpu0 1393 87 149 982 5 12 0 0 0 0\n\
cpu1 3312 269 435 2717 18 11 0 0 0 0\n\
intr 1462898 0 9 0 0 0 0 0 0 1\n\
ctxt 2750148\n\
btime 1730800000\n";

pub const PROC_MEMINFO: &[u8] = b"MemTotal:       16384000 kB\n\
MemFree:         1024000 kB\n\
MemAvailable:    8192000 kB\n\
Buffers:          512000 kB\n\
Cached:          4096000 kB\n\
SwapCached:            0 kB\n\
Active:          6000000 kB\n";

pub const PROC_MEMINFO_OLD: &[u8] = b"MemTotal:       16384000 kB\n\
MemFree:         1024000 kB\n\
Buffers:          512000 kB\n\
Cached:          4096000 kB\n";

pub const PROC_NET_DEV: &[u8] = b"Inter-|   Receive                                                |  Transmit\n \
face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed\n    \
lo: 1000000    5000    0    0    0     0          0         0  1000000    5000    0    0    0     0       0          0\n  \
eth0: 987654321 654321    0    0    0     0          0       100 123456789  234567    0    0    0     0       0          0\n\
docker0:    5000      40    0    0    0     0          0         0     9000      50    0    0    0     0       0          0\n";

pub const PROC_UPTIME: &[u8] = b"350735.47 234388.90\n";

pub const PROC_LOADAVG: &[u8] = b"0.20 0.18 0.12 1/405 9999\n";

pub const DF_LINUX: &[u8] = b"Filesystem        1-blocks         Used    Available Capacity Mounted on\n\
udev            8330133504            0   8330133504       0% /dev\n\
tmpfs           1675055104      1445888   1673609216       1% /run\n\
/dev/nvme0n1p2 500000000000 125000000000 375000000000      25% /\n\
/dev/nvme0n1p1    536870912    268435456    268435456      50% /boot/efi\n\
/dev/sdb1     1000000000000 100000000000 900000000000      10% /mnt/Backup Drive\n\
overlay        500000000000 125000000000 375000000000      25% /var/lib/docker/overlay2/abc/merged\n\
tmpfs           8375275520            0   8375275520       0% /dev/shm\n";

pub const SYSCTL: &[u8] = b"17179869184\n16384\n{ 1.23 1.45 1.67 }\n\
{ sec = 1730800000, usec = 123456 } Tue Nov  5 09:46:40 2024\n";

pub const VM_STAT: &[u8] = b"Mach Virtual Memory Statistics: (page size of 16384 bytes)\n\
Pages free:                               12345.\n\
Pages active:                            234567.\n\
Pages inactive:                          222222.\n\
Pages speculative:                         1000.\n\
Pages throttled:                              0.\n\
Pages wired down:                         88888.\n\
Pages purgeable:                           5000.\n\
\"Translation faults\":                 123456789.\n\
Pages copy-on-write:                    1234567.\n\
File-backed pages:                       100000.\n\
Anonymous pages:                         150000.\n\
Pages stored in compressor:              300000.\n\
Pages occupied by compressor:            120000.\n";

pub const DF_MACOS: &[u8] =
    b"Filesystem     1024-blocks      Used Available Capacity  Mounted on\n\
/dev/disk3s1s1   500000000 125000000 300000000    30%    /\n\
devfs                  215       215         0   100%    /dev\n\
/dev/disk3s6     500000000   1048576 300000000     1%    /System/Volumes/VM\n\
/dev/disk3s5     500000000 250000000 200000000    56%    /System/Volumes/Data\n\
map auto_home            0         0         0   100%    /System/Volumes/Data/home\n";

pub const NETSTAT: &[u8] = b"Name       Mtu   Network       Address            Ipkts Ierrs     Ibytes    Opkts Oerrs     Obytes  Coll\n\
lo0        16384 <Link#1>                        100000     0   50000000   100000     0   50000000     0\n\
lo0        16384 127           127.0.0.1         100000     -   50000000   100000     -   50000000     -\n\
lo0        16384 ::1/128       ::1               100000     -   50000000   100000     -   50000000     -\n\
en0        1500  <Link#14>   a4:83:e7:11:22:33  9876543     0 9876543210  5432100     0 1234567890     0\n\
en0        1500  192.168.1/24  192.168.1.20      9876543     - 9876543210  5432100     - 1234567890     -\n\
en0        1500  fe80::1c2b:3d4e fe80::1c2b:3d4e 9876543     - 9876543210  5432100     - 1234567890     -\n\
utun0      1380  <Link#16>                            0     0          0       10     0        900     0\n\
en5*       1500  <Link#9>    aa:bb:cc:00:11:22        0     0          0        0     0          0     0\n";

pub const TOP: &[u8] = b"Processes: 500 total, 3 running, 497 sleeping, 2000 threads\n\
2024/11/05 09:46:40\n\
Load Avg: 1.23, 1.45, 1.67\n\
CPU usage: 8.33% user, 12.50% sys, 79.17% idle\n\
SharedLibs: 600M resident, 100M data, 50M linkedit.\n\
Processes: 500 total, 3 running, 497 sleeping, 2000 threads\n\
2024/11/05 09:46:41\n\
Load Avg: 1.23, 1.45, 1.67\n\
CPU usage: 5.26% user, 10.52% sys, 84.22% idle\n\
SharedLibs: 600M resident, 100M data, 50M linkedit.\n";
