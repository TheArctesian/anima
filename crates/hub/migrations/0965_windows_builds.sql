-- WIN2: native Windows builders and signed NSIS releases use the existing build queue.
alter table builds.builders drop constraint builders_os_check;
alter table builds.builders add constraint builders_os_check
    check (os in ('linux', 'macos', 'windows'));
alter table builds.jobs drop constraint jobs_target_check;
alter table builds.jobs add constraint jobs_target_check
    check (target in ('linux-x86_64', 'darwin-universal', 'windows-x86_64'));
