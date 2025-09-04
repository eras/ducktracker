Copyright Erkki Sepppälä <flux@inside.org>

# DuckTracker

DuckTracker is an alternative backend to
[Hauk](https://github.com/bilde2910/Hauk), which is an open source
mobile phone tracker app with clients for Android and iOS.

The idea of DuckTracker is to facilitate a large group of people
visiting some place to see where each others are going. This happens
by configuring the Hauk client to publish certain tags.

One such tag could be "museum", used by the subgroup of people
visiting museums. Then when using the web client, one can see the
location of the group, assuming the user knows the keyword "museum" to
enter to the client. So normal tags are private, they will be shared
only to others aware of the same tag.

Then there are public tags, which start with the string "pub". Shares
involving "pub" are pushed to new and existing clients when they appear.

# Compilation

1) `scripts/export-models-types.sh --release` to generate frontend/bindings
2) (cd frontend && npm install && npm run build)
3) `cargo build --release`

# Configure

## ducktracker
Enter user:pass to ducktracker.passwd, e.g.

```
hello:world
```

(encryption is not supported yet)

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
