# ssnfc

Really basic tool to control the fan speed of my Nvidia 3060 graphics card on Linux. Not tested on other hardware but could still work. You also need `nvidia-settings` installed.

## Usage

```
ssnfc --config "path/to/config.json"
```

### Using systemd

You can create a simple service using systemd. I use gdm so it probably needs some change for other environments.

```
[Unit]
Description=ssnfc service
After=gdm.target

[Service]
ExecStart=/usr/local/bin/ssnfc --config /etc/ssnfc.json

[Install]
WantedBy=multi-user.target
```
