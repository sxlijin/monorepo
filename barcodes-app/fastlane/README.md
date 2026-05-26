fastlane documentation
----

# Installation

Make sure you have the latest version of the Xcode command line tools installed:

```sh
xcode-select --install
```

For _fastlane_ installation instructions, see [Installing _fastlane_](https://docs.fastlane.tools/#installing-fastlane)

# Available Actions

## iOS

### ios metadata

```sh
[bundle exec] fastlane ios metadata
```

Push App Store metadata + screenshots (no binary) to App Store Connect

### ios release

```sh
[bundle exec] fastlane ios release
```

Build the app (gym) and upload the binary to App Store Connect (deliver)

### ios metadata_verify

```sh
[bundle exec] fastlane ios metadata_verify
```

Validate the metadata locally without uploading (no API key needed)

----

This README.md is auto-generated and will be re-generated every time [_fastlane_](https://fastlane.tools) is run.

More information about _fastlane_ can be found on [fastlane.tools](https://fastlane.tools).

The documentation of _fastlane_ can be found on [docs.fastlane.tools](https://docs.fastlane.tools).
