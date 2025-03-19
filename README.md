# ISO-RS

A rust implementation of the ISO-9660 filesystem.
Currently there are a few limitations:

- Only supports spec compliant filesystems, and no extensions.
- Sector size has to be 2KiB (2048 bytes), this is the common size, and should be supported by most filesystems.
