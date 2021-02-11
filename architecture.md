# Architecture for the Tunneler-Core
## Server
### Client
A Client on the Server-Side is some other instance of a tunneler-compliant software
that is ready to receive requests to forward them to whatever was specified.
One example for such a client would be something that implements the client-Module
from this crate.

### User
A user is then the actual request or connection thereof. These can change very
rapidly and the handling of these should also be of the highest priority and be
done as fast as possible.

### Client-Manager
The Manager to actually handle and keep track of all the connected Clients

## Message
### Message
The Message itself to be send to the other Side of the communication, so Server->Client
or Client->Server.
This is composed of a Header + the body of the Message

### Header
The Header of the message contains some crucial information about the following message,
like what type of message, the ID of the connection and importantly, how big the message
is.

## Connections
### Connection
(This is slowly being removed)
Represents a single Connection which is basically just a tokio-TCPStream with some
nice helper features

### Destination
A nice wrapper that stores a Destination as an address and allows you to easily create
a connection to said destination.
