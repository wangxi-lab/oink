# Oink LAN Only privacy notice

Oink LAN Only is a modified distribution of upstream Oink. The application
does not send crash or performance reports, contact an update feed, download
sponsor content or open web URLs from its desktop interface.

On directly connected private IPv4 subnets, paired devices can exchange keyboard
and mouse events, clipboard data, files and other enabled sharing features over
encrypted links. LAN discovery broadcasts basic device information, including
the device name and service port, before pairing. Local diagnostics can contain
system and connection details. Exported diagnostics are written to local storage.

Install the Windows x64 MSI on both approved devices and pair them on the same
private IPv4 subnet. If discovery is unavailable, enter the other device's
private IPv4 address and the QUIC service port shown in its Oink interface.
WebView2 is a prerequisite. The MSI does not download it during setup.

See [SECURITY-REVIEW.md](SECURITY-REVIEW.md) for the scope of verification and
deployment boundary.
