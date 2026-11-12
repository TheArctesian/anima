//! Output captured from a Linux 6.x host and a macOS 15 host. The GPU outputs say in their own
//! comments which are captured and which are built from the documented format.

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

/// Captured on pop-os (the operator's Linux machine, one RTX A5000) on 2026-11-07 with the
/// design spec §6.6 `nvidia-smi` command, unedited.
pub const NVIDIA_SMI: &[u8] = b"0, NVIDIA RTX A5000, 12, 735, 24564, 61, 30.52\n";

/// Constructed from the documented format: two GPUs, one with a comma in its name.
pub const NVIDIA_SMI_TWO: &[u8] = b"0, NVIDIA GeForce RTX 4090, 97, 20480, 24564, 71, 412.33\n\
1, Tesla T4, Quadro, Mixed, 3, 1024, 15360, 45, 28.10\n";

/// Constructed from the documented format: the fields a laptop GPU does not give.
pub const NVIDIA_SMI_NOT_AVAILABLE: &[u8] = b"0, NVIDIA GeForce MX150, 5, 100, 2048, 40, [N/A]\n\
1, NVIDIA Grid, [Not Supported], [Not Supported], [Not Supported], 50, [Not Supported]\n";

/// Constructed from the documented format: `amdgpu` sysfs file contents of one card.
pub const AMD_GPU_BUSY_PERCENT: &[u8] = b"37\n";
pub const AMD_VRAM_USED: &[u8] = b"2147483648\n";
pub const AMD_VRAM_TOTAL: &[u8] = b"17163091968\n";

/// Constructed from the documented format (`intel_gpu_top -J`, newer versions): a JSON array.
pub const INTEL_GPU_TOP_ARRAY: &[u8] = b"[\n\
{\n\"period\": {\"duration\": 1000.1, \"unit\": \"ms\"},\n\
\"power\": {\"GPU\": 0.5, \"Package\": 6.2, \"unit\": \"W\"},\n\
\"engines\": {\"Render/3D/0\": {\"busy\": 4.0, \"sema\": 0.0, \"wait\": 0.0, \"unit\": \"%\"}, \
\"Video/0\": {\"busy\": 0.0, \"unit\": \"%\"}}\n\
},\n\
{\n\"period\": {\"duration\": 1000.4, \"unit\": \"ms\"},\n\
\"power\": {\"GPU\": 1.5, \"Package\": 7.0, \"unit\": \"W\"},\n\
\"engines\": {\"Render/3D/0\": {\"busy\": 21.5, \"unit\": \"%\"}, \
\"Video/0\": {\"busy\": 33.25, \"unit\": \"%\"}}\n\
}\n]\n";

/// Constructed from the documented format (older `intel_gpu_top -J`): objects separated by
/// commas, no brackets, the last one cut off.
pub const INTEL_GPU_TOP_STREAM_CUT: &[u8] =
    b"{\"engines\": {\"Render/3D/0\": {\"busy\": 10.0}}},\n\
{\"engines\": {\"Render/3D/0\": {\"busy\": 60.0}}},\n\
{\"engines\": {\"Render/3D/0\": {\"bus";

/// Constructed from the documented format: two accelerators, as `ioreg -r -d 1 -c
/// IOAccelerator` prints them (properties trimmed to the ones read and a few neighbours).
pub const IOREG_ACCELERATOR: &[u8] = b"+-o AGXAcceleratorG14X  <class AGXAcceleratorG14X, id 0x100000436, registered, matched, active, busy 0 (0 ms), retain 28>\n\
  {\n\
    \"IOClass\" = \"AGXAcceleratorG14X\"\n\
    \"model\" = \"Apple M2 Pro\"\n\
    \"gpu-core-count\" = 19\n\
    \"PerformanceStatistics\" = {\"Alloc system memory\"=1234567168,\"Device Utilization %\"=17,\"In use system memory\"=987654321,\"Renderer Utilization %\"=12}\n\
  }\n\
\n\
+-o AGXAcceleratorG14X  <class AGXAcceleratorG14X, id 0x100000437, registered, matched, active, busy 0 (0 ms), retain 28>\n\
  {\n\
    \"model\" = \"Apple M2 Pro\"\n\
    \"PerformanceStatistics\" = {\"Device Utilization %\"=3,\"In use system memory\"=1000}\n\
  }\n\
\n";

/// Constructed from the operator's `lsblk` on pop-os (2026-11-12) in the form of util-linux 2.39
/// `lsblk -J -b -o NAME,TYPE,SIZE,ROTA,TRAN,MOUNTPOINTS`: a 22 TB and an 8 TB disk with one
/// unmounted partition each, compressed swap, an unmounted `NVMe` drive, and the system `NVMe` drive
/// with `/` on LVM in a LUKS container and swap in another (MET-027).
pub const LSBLK_POP_OS: &[u8] = br#"{
   "blockdevices": [
      {"name":"sda", "type":"disk", "size":24000277250048, "rota":true, "tran":"sata", "mountpoints":[null],
         "children": [
            {"name":"sda1", "type":"part", "size":24000275152896, "rota":true, "tran":null, "mountpoints":[null]}
         ]
      },
      {"name":"sdb", "type":"disk", "size":8001563222016, "rota":true, "tran":"sata", "mountpoints":[null],
         "children": [
            {"name":"sdb1", "type":"part", "size":8001561124864, "rota":true, "tran":null, "mountpoints":[null]}
         ]
      },
      {"name":"zram0", "type":"disk", "size":8589934592, "rota":false, "tran":null, "mountpoints":["[SWAP]"]},
      {"name":"nvme0n1", "type":"disk", "size":1000204886016, "rota":false, "tran":"nvme", "mountpoints":[null],
         "children": [
            {"name":"nvme0n1p1", "type":"part", "size":536870912, "rota":false, "tran":"nvme", "mountpoints":[null]},
            {"name":"nvme0n1p2", "type":"part", "size":999666221056, "rota":false, "tran":"nvme", "mountpoints":[null]}
         ]
      },
      {"name":"nvme1n1", "type":"disk", "size":1000204886016, "rota":false, "tran":"nvme", "mountpoints":[null],
         "children": [
            {"name":"nvme1n1p1", "type":"part", "size":1073741824, "rota":false, "tran":"nvme", "mountpoints":["/boot/efi"]},
            {"name":"nvme1n1p2", "type":"part", "size":4294967296, "rota":false, "tran":"nvme", "mountpoints":["/recovery"]},
            {"name":"nvme1n1p3", "type":"part", "size":990540488704, "rota":false, "tran":"nvme", "mountpoints":[null],
               "children": [
                  {"name":"cryptdata", "type":"crypt", "size":990523711488, "rota":false, "tran":null, "mountpoints":[null],
                     "children": [
                        {"name":"data-root", "type":"lvm", "size":990519517184, "rota":false, "tran":null, "mountpoints":["/"]}
                     ]
                  }
               ]
            },
            {"name":"nvme1n1p4", "type":"part", "size":4294967296, "rota":false, "tran":"nvme", "mountpoints":[null],
               "children": [
                  {"name":"cryptswap", "type":"crypt", "size":4294967296, "rota":false, "tran":null, "mountpoints":["[SWAP]"]}
               ]
            }
         ]
      }
   ]
}"#;

/// Constructed from the documented format of util-linux before 2.33, `-o ...,MOUNTPOINT`:
/// every value a string, one mount point per device.
pub const LSBLK_OLD: &[u8] = br#"{
   "blockdevices": [
      {"name": "sda", "type": "disk", "size": "500107862016", "rota": "1", "tran": "sata", "mountpoint": null,
         "children": [
            {"name": "sda1", "type": "part", "size": "500106813440", "rota": "1", "tran": null, "mountpoint": "/"}
         ]
      },
      {"name": "sr0", "type": "rom", "size": "1073741312", "rota": "1", "tran": "sata", "mountpoint": null},
      {"name": "vda", "type": "disk", "size": "21474836480", "rota": "", "tran": null, "mountpoint": "/srv"}
   ]
}"#;

/// Constructed from the documented format of `/proc/diskstats` (kernel 5.5 and later, 20
/// columns): two drives, a partition, a device-mapper volume and a loop device.
pub const PROC_DISKSTATS: &[u8] = b"   7       0 loop0 52 0 2180 11 0 0 0 0 0 40 11 0 0 0 0 0 0\n\
 259       0 nvme1n1 120000 3000 9000000 40000 80000 9000 6000000 70000 0 50000 110000 0 0 0 0 900 300\n\
 259       1 nvme1n1p1 300 0 20000 100 2 0 16 1 0 120 101 0 0 0 0 0 0\n\
   8       0 sda 4000 10 800000 30000 100 5 20000 4000 0 21000 34000 0 0 0 0 0 0\n\
 253       0 dm-0 110000 0 8800000 39000 89000 0 6000000 90000 0 52000 129000 0 0 0 0 0 0\n";
