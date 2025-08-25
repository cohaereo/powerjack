# `powerjack-demo`

A parser for the Source 1 DEM replay container format.

Note that this crate does _not_ provide any functionality for parsing the actual network packets within the DEM files.

This crate supports all commands from Team Fortress 2:

- `dem_signon` -> `Command::SignOn` (raw bytes only)
- `dem_packet` -> `Command::Packet` (raw bytes only)
- `dem_synctick` -> `Command::SyncTick`
- `dem_consolecmd` -> `Command::ConsoleCmd`
- `dem_usercmd` -> `Command::UserCmd`
- `dem_datatables` -> `Command::DataTables`
- `dem_stop` -> `Command::Stop`
- `dem_stringtables` -> `Command::StringTables`
