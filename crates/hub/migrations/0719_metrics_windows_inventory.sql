-- WIN2: native Windows daemons report the same bounded tool inventory as other platforms.
alter table metrics.inventories drop constraint inventories_os_check;
alter table metrics.inventories add constraint inventories_os_check
    check (os in ('macos', 'linux', 'windows'));
