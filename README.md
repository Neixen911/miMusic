# miMusic

miMusic is a high-performance audio player designed specifically for lightweight systems. Developed entirely in Rust, it combines a TUI with complete playlist management and an integrated download system. Its architecture aims to be optimised and responsive to ensure smooth audio playback while consuming minimal system resources.

Here’s a preview of miMusic’s terminal interface:

![miMusic-TUI](https://github.com/user-attachments/assets/70bd4427-19f4-489a-81c8-d6020c93d663)
*🚧 (miMusic is currently under active development) 🚧*

## Installation

To install miMusic and be capable of use it, clone the repository and run this instructions into your Terminal:
```bash
sudo apt-get install rustup libasound2-dev pkgconf
rustup default nightly
```

Now, to run the app, you will need to run ```cargo run --bin miMusic```.
You will see the interface and now, you are capable to use it on your own with keyboards shortcuts in the bottom of the app.

I'm pretty open to suggestions and be extremely happy to know that the app would be helping you ! Enjoy it !

## Main Features

- TUI Interface
- Playback Controls
- **Songs Library**
- Playlist Management
- Integrated Download System
- Normalized Audio System

## Technical Information

**Optimized Performance** : Memory consumption of approximately 15MB currently. The architecture aims to guarantee minimal CPU and memory usage. Performances are currently being optimized.

**Instant Startup** : Fast launch time thanks to optimized architecture. Startup in a few milliseconds.

**Enhanced Stability** : The simplified architecture aims to ensure reliable operation on aging hardware or in constrained environments, guaranteeing an uninterrupted audio experience.

**Offline Operation** : Complete autonomy - only download mode requires an Internet connection to retrieve MP3 files. Once downloaded, all your music is accessible entirely offline.

**Storage Requirements** : Plan storage space suitable for your music library (approximately 3-5MB per song). The player downloads and stores all MP3 files locally to ensure smooth playback without network dependency.

## Upcoming Features

- [ ] Audio waveform visualization
- [ ] Extended ID3 tags support
- [ ] Exclusive playback per device
- [ ] Non-regression automated tests

------------------------------------

    MIT © Neixen911
