# File type association

`windows-file-association.reg` registers `.jsd` (Jiffshot documents) so the
shell opens them with `jiffshot.exe`, which takes the file as a command-line
argument (a running Jiffshot opens it in its editor). It is packaging
metadata, not an installer: an installer writes the same keys, or
`reg import` applies them for local testing. Replace the install path in it
first.
