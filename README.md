I use this to route notifications from remote machines to the correct desktop.

A central instance `notif --server` receives notifications and forwards them to the latest registered notifier.
A notifier is run on a desktop machine and shows the notifications.
`notif --send` is used in place of a local `notify-send`.


Example:
- remote machine A: `notif --server`
- laptop1: `notif --notifier laptop1`
- laptop2: `notif --notifier laptop2`
- remote machine B: `notif --send normal "something noteworthy" "just happened"`


Laptop2's notifier will receive & feed the notification to the desktop's notification manager and you'll see "*@machineB: something noteworthy* just happened".


On a desktop I use it like this: this ensures that the machine that has most recently unlocked X session will receive the notifications.
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


I use it on a private VPN, notifications are sent in cleartext.
