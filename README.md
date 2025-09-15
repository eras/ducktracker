Copyright Erkki Sepppälä <flux@inside.org>

# DuckTracker

DuckTracker is an alternative backend to
[Hauk](https://github.com/bilde2910/Hauk), which is an open source
mobile phone tracker app with clients for Android and iOS.

The idea of DuckTracker is to facilitate a large group of people
visiting some place to see where each others are going. This happens
by configuring the Hauk client to publish certain tags.

One such tag could be "museum", used by the subgroup of people
visiting museums. If the share is made as a public one, which is the
default, then it will appear to others when they just select the tag
(or have no tags selected).  If the share is private, the others will
need to know to add the "museum" tag to their clients to track it.

By default the tags are private. To make a publick share, you need to
use `pub:` (or `public:`) prefix. So e.g. `pub:museum`. This tag will
be pushed to all clients, which may then choose to show that as well
or not.

Another example:

`pub:everyone,flux-at-the-bar`

would result in two shares: `everyone` would be public, while
`flux-at-the-bar` would be private.

By default, if the share id is left empty in the Hauk mobile client, a
random private id is generated for the user.

# Tag formats

| Tag/Syntax        | Description                                                                 |
|-------------------|-----------------------------------------------------------------------------|
| `private:tagname` | Private tag, only sent to clients that know it already                      |
| `priv:tagname`    | (Same as above)                                                             |
| `tagname`         | (Same as above)                                                             |
| `public:tagname`  | Public tag, pushed to clients that ask for all tags                         |
| `pub:tagname`     | (Same as above)                                                             |
| `points:42`       | Not a tag; set the maximum number of points to store for this share session |

# Compilation

1) `scripts/export-models-types.sh --release` to generate frontend/bindings
2) (cd frontend && npm install && npm run build)
3) `cargo build --release`

# Configure

## ducktracker
Enter user:pass to ducktracker.passwd, e.g.

```
hello:world
hello2:$2b$12$7KCLyegP2KL.9X6LpKiLh.5ybmH5KWFFDCXD2KRANBfUmfqQ5cDv.
```

For encrypting passwords use bcrypt. You may use `htpasswd -B
ducktracker.passwd username` to add new users to the file (available
in Debian's `apache2-utils`). You need to restart the server to reload
it.

## Hauk

Use the user/pass you've configured to the password file.

In the "preferred link id" field list the tags you want to share,
separated by comma, e.g. `pub,museum`.

# Run

`cargo run`

# Using the web interface

Browse to the server, enter user name and password, start using! Use
the interface to add the private tags you know of. Those and the
user/password will be persisted in localstorage. (The system currently
doesn't support long-running tokens, so it instead stores credentials
for ease of use.)

If there are no tags selected, then all public tags will be shown. If
any of the tags is selected, then the selected tags function as a
filter.

# Privacy

## The --box-coords -feature

As a particular privacy feature, or testing feature, one can start the
server with `--box-coords lat1,lng1,lat2,lng2`, e.g. `--box-coords
59.432465,24.744732,59.441459,24.762416`. This will wrap the input
data to be in that box before further processing. This way no clients
can get the original coordinates. Do note though that with some data
analysis and guesswork it may still be possible to recover the origin
data.
