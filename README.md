I use this to route notifications from remote machines to the current desktop I'm using.

A central instance `notif route` receives notifications and forwards them to the latest registered notifier.
A notifier is run with `notif notify` on a desktop machine and shows the notifications.
`notif send` is used in place of a local `notify-send`.


Example:
- remote machine A: `notif route`
- laptop1: `notif notify`
- laptop2: `notif notify`
- remote machine B: `notif send -u critical "something noteworthy" "just happened"`


Laptop2's notifier will receive & feed the notification to the desktop's notification manager and you'll see "*@machineB: something noteworthy* just happened".


For this magic to happen notif looks for a config file in `~/.notif`, `/etc/notif`, or as an argument `notif -c <file>`.

```sh
notif generate topo 10.99.0.1:9961 10.99.0.1:9962 5
```
This will generate config files for 5 clients & a server with curve certificates for each.


On a desktop I use `notif notify` with this: this ensures that the machine that has most recently unlocked X session will receive the notifications.
```sh
xscreensaver-command -watch | while read xs; do
  case "$xs" in
    LOCK*)
      killall -s SIGUSR1 dunst   # pause dunst so notifications don't appear over xscreensaver
      ;;
    UNBLANK*)
      killall -s SIGUSR2 dunst   # resume dunst
      killall -s SIGHUP notif    # have notif send a SEIZE message to become the notifier.
      ;;
  esac
done
```

I use this with this kind of things:
- [emerge notif](misc/emerge_notif-prompts.patch)
- [zbell](https://github.com/Wonko7/conf-zsh/blob/master/zbell.zsh#L69)


I use this on a private VPN, notifications are sent in cleartext.
