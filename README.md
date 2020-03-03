I use this to route notifications from remote machines to the current desktop I'm using.

A central instance `notif route` receives notifications and forwards them to the currently active notifier.
A notifier is run with `notif notify` on a desktop machine and displays received notifications.
`notif send` is used in place of a local `notify-send`.


Example:
- remote machine A: `notif route`
- laptop1: `notif notify`
- laptop2: `notif notify`
- remote machine B: `notif send -u critical "something noteworthy" "just happened"`


Laptop2's notifier will receive & feed the notification to the desktop's notification manager and you'll see "*@machineB: something noteworthy* just happened".
Send a SIGUSR2 to laptop1's notifier and it will receive future notifications.


For this to happen notif looks for a config file in `~/.notif`, `/etc/notif`, or as an argument `notif -c <file>`: [localhost example](misc/notif-example-conf.yaml).
Notif can generate config files for multiple hosts: for 5 clients & a server with curve certificates for each:
```sh
notif generate topo 10.99.0.1:9961 10.99.0.1:9962 5
```


On a desktop I use `notif notify` with this script: this ensures that the machine that has most recently unlocked X session will receive the notifications.
```sh
xscreensaver-command -watch | while read xs; do
  case "$xs" in
    LOCK*)
      # pause dunst so notifications don't appear over xscreensaver
      # pause notif so notifications queue up on server & will be routed later (maybe to another desktop)
      svc-s6 -1 $s6/notif || killall -s SIGUSR1 dunst
      svc-s6 -1 $s6/dunst || killall -s SIGUSR1 notif
      ;;
    UNBLANK*)
      # have notif send a SEIZE message to become the active notifier. resume dunst.
      svc-s6 -2 $s6/notif || killall -s SIGUSR2 dunst
      svc-s6 -2 $s6/dunst || killall -s SIGUSR2 notif
      ;;
  esac
done
```

I use this with this kind of things:
- [emerge notif](misc/emerge_notif-prompts.patch)
- [zbell](https://github.com/Wonko7/conf-zsh/blob/master/zbell.zsh#L69)
