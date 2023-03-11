<div align="center">
  <a href="https://discord.shaybox.com">
    <img alt="Discord" src="https://img.shields.io/discord/824865729445888041?color=404eed&label=Discord&logo=Discord&logoColor=FFFFFF">
  </a>
  <a href="https://github.com/shaybox/vrc-yt-proxy/releases/latest">
    <img alt="Downloads" src="https://img.shields.io/github/downloads/shaybox/vrc-yt-proxy/total?color=3fb950&label=Downloads&logo=github&logoColor=FFFFFF">
  </a>
</div>

# VRC-YT-Proxy

Play YouTube videos on Quest in VRChat

Similar to [vroxy](https://github.com/techanon/vroxy) but written in Rust with ease of use in mind

## Usage:

[You can use my public instance](https://shay.loan)

## Self-Hosting:

You can self-host your own instance using the [latest release](https://github.com/ShayBox/VRC-YT-Proxy/releases/latest)

The default binding address and port are intended for use behind a reverse proxy,  
Should you not want to use a reverse proxy and accept connections from remote hosts,  
You can change the address and port using the `ADDR` and `PORT` environment variables.

The default binding address `127.0.0.1` only accepts local hosts, using `0.0.0.0` accepts all hosts.
The default binding port is `8000`, ports below 1024 usually require root privilege on Linux.  
