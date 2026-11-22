# Oink LAN Only security review

## Runtime network boundary

The Windows x64 distribution permits peer traffic on directly connected private
IPv4 subnets (including link-local Ethernet and local loopback). QUIC checks
destinations and drops disallowed datagrams before decoding. Manual lookup,
discovery replies and TCP file transfers check peer addresses. The installer
limits inbound rules to the local subnet. Discovery uses private subnet
broadcasts and does not start mDNS. Existing settings, advertised endpoints
and persisted addresses do not grant permission to reach public IP addresses.

Telemetry has no production HTTP client or configured endpoint. Online update
trust and HTTP transfers are disabled in the release build. The explicitly
enabled updater test transport uses only a literal loopback origin. Sponsor
downloads and the native command to open browser URLs have been removed.
The desktop WebView limits navigation to packaged pages and blocks pop-ups.
Windows in-app installer drop and vendor microphone driver installation are
disabled. Use an IT-approved, locally staged MSI to update the app. The
installer checks for an already installed WebView2 runtime.

An IP prefix check cannot identify the owner of a network or attest to its
routing. To enforce a company policy, an administrator should permit only the
approved peer IPs on the approved interface and deny other egress for the
installed app and its helper processes via Windows Firewall or equivalent
controls. Windows and WebView2 can have their own networking policies.
Paired peers can receive keyboard events, clipboard contents and files and are
trusted to handle them according to company policy.

## Source and dependency checks

The fork started from commit `3e663a654b328193cef072972d4f26e32846cd09`,
which matched upstream Oink’s `v0.3.7` tag.

- Reviewed runtime HTTP clients, web navigation, socket connects and UDP
  sends, native process launch and installer service registration paths. The
  upstream sponsor, report and update requests were explicit application
  features. The inspected paths showed no unexplained external uploader or
  unexpected executable entry point.
- The base checkout's tracked file headers showed no PE, ELF, Mach-O or
  WebAssembly executables. Catalog package fixtures are text. This check
  covers repository files, with build-time dependencies and the generated MSI
  covered separately.
- The npm production advisory check reported zero known vulnerabilities.
  RustSec found RUSTSEC-2026-0592 in the upstream URL dependency. The fork
  updates `url` to 2.5.9 and `rustix` to 1.1.6. The post-update audit
  reported no vulnerability entries and retained maintenance warnings for
  `chrono` and `proc-macro-error` and an unsoundness warning for Linux's `glib`.
  Advisory checks and static inspection have limited coverage.
- The release workflow builds from the tagged source, stages the unsigned
  Windows MSI and a SHA-256 checksum, and requires a successful Microsoft
  Defender scan of the staged installer and payload before publishing.

The review does not certify the absence of malware. Validate the installed
binary and confirm network behavior with two approved Windows PCs before
deploying it to managed devices. This distribution retains the upstream
Windows installation identity and service. It is intended to replace a
compatible upstream install rather than run alongside it.
