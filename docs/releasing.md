# Release builds

The release workflow is [`.github/workflows/release.yml`](../.github/workflows/release.yml).
It builds these unsigned packages:

| Platform | Release files |
| --- | --- |
| Windows | MSI installer, NSIS setup EXE, portable EXE, portable ZIP |
| Linux | Debian package (`.deb`), AppImage |

## Version and tag

Set the product version in `src-tauri/Cargo.toml` (`[package].version`) and
keep `src-tauri/tauri.conf.json`'s `version` equal to it. The workflow reads
Cargo's version and fails if those values differ. It does not hardcode a
release version.

To publish from a tag, push the exact matching tag, such as `v1.2.2` for
version `1.2.2`. A tag that does not match the version in the checked-out code
fails before building. Alternatively, run **Build and publish releases** with
`workflow_dispatch`; the workflow uses the current code version and creates
the corresponding `v<version>` release tag.

The workflow waits for both platform builds before publishing one GitHub
release with generated release notes and all package files attached. Bump the
version in both source metadata files to publish a distinct release.

The Windows and Linux jobs cache Cargo's dependency registry and compiled
dependency artifacts. The first run after a dependency or toolchain change
still needs a cold build; later runs can reuse compatible cached artifacts.

## Build requirements and runtime notes

- Windows builds MSI and NSIS using the native Windows runner. The portable
  ZIP contains the executable, license, and README; the standalone EXE is also
  published. These are not self-contained WebView2 distributions. Windows
  systems need the Microsoft Edge WebView2 Runtime, which is normally present
  on supported Windows 10 and Windows 11 systems. The installers use Tauri's
  configured WebView2 install behavior.
- Linux packages are built on Ubuntu 22.04. AppImage provides a single-file
  package, while the Debian package is for Debian-derived systems. Tauri's
  WebKitGTK runtime requirements still apply on Linux; the AppImage does not
  bundle WebKitGTK.
- These packages are not code-signed by this workflow. Signing credentials and
  signing configuration have not been added.

For a local Windows build, follow the commands in the root [README](../README.md).
