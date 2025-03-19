# ISO-RS

A rust implementation of the ISO-9660 filesystem.
Currently there are a few limitations:

- Only supports spec compliant filesystems, and no extensions.
- Sector size has to be 2KiB (2048 bytes), this is the common size, and should be supported by most filesystems.

This project also contains the specification for the ISO-9660 filesystem, which is a work in progress.

## Current Progress

- [x] Parsing of the ISO-9660 filesystem
    - [x] Parsing of the primary volume descriptor
    - [x] Parsing of the volume descriptor list
    - [x] Parsing of the path table
    - [x] Parsing of the root directory
    - [x] Parsing of the directory records
- [ ] Writing of the ISO-9660 filesystem
    - [x] Basic support for writing ISO-9660 filesystems (only to the root directory, but arbituary size)
    - [ ] Support for writing to root directory bigger than 1 sector
    - [ ] Support for writing to the path table
    - [ ] Support for writing directories
    - [ ] Support for writing files in non-root directories
- [ ] Support for El Torito booting
   - [ ] Support for parsing El Torito Boot Records
   - [ ] Support for loading Boot files
