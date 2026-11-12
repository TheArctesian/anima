-- R-113 (RA-040 and RA-041 deferred, RA-008): the hub records no session content. Drops the
-- recordings table of 0660, and with it the paging indexes of 0661 and the column grants 0660
-- gave the runtime role. No tunnel ever wrote a recording (WP-48 T7 and T8 were cancelled),
-- so no row is lost. The session rows stay (RA-008).

drop table sessions.recordings;
