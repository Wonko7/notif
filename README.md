A *sender* sends notifications to the server, and multiple senders can exist and be active.
A *notifier* receives notifications from the server and shows them to the user.
A *server* forwards notifications from senders to the current notifier.

A single notifier is active at any given time. Notifier sends an ID on the
server's yield socket to be active.

Example:
- `notifier --server`
- `notifier --notifier kekette`
- `notifier --sender`

Notifier kekette receives the notifications.
now add a new notifier:
`notifier --notifier p3n1s`
p3n1s receives the notifications and kekette does not.
