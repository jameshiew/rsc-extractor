# rsc-extractor

> ⚠️ This project is experimental and shared as-is.
> Expect rough edges, vibe-coded logic, and unreviewed code — use at your own risk!

A tool for extracting resources from [BYOND](https://www.byond.com) `.rsc` files, such as `.dmi` and `.midi` files. Makes a best effort to extract even in cases where a resource has no filename, or there are duplicates.

Huge credit to <https://github.com/20kdc/byond-data-docs> as a resource on the file format.

## Installation

```shell
cargo install --locked --git https://github.com/jameshiew/rsc-extractor rsc-extractor
```

## Quickstart

```shell
rsc-extractor some.rsc  # prints analysis of resources in some.rsc
rsc-extractor some.rsc --out dir/  # extracts all valid resources to dir/
rsc-extractor --help  # to see other options
```

## Testing

There are [xtasks](https://github.com/matklad/cargo-xtask) for downloading and extracting sample projects from the [BYOND Preservation Project](https://archive.org/details/ByondPreservationProject) on the Internet Archive as it was on 2 November 2025 (SHA256 hash `80f5d52c169450429b863ed3088fd640f9a4978472081a352223fd63d9795490`).

```shell
cargo xtask download-bpp  # to workspace/ByondPreservationProject.zip
cargo xtask unzip-bpp  # to workspace/projects/...
cargo xtask analyze-all  # print analysis of all .rsc files as a sanity check
cargo xtask extract-all  # to directories under workspace/extracted/...
```
