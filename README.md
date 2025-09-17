Wrenlet is a set of rust bindings around the scripting language
[Wren](https://wren.io), with the goal of providing safe and easy to use
bindings.

# Features

`wrenlet` is currently a rather barebones library. As of this moment, it only
supports Wren v0.4.0, and even then, it does not support all the features of the
C implementation.

Primarily, foreign methods and classes are not implemented, although this is a
high priority. Additionally, `wrenlet` only supports passing or returning
immutable or non-reference types.
