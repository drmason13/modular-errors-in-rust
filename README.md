Example demonstrating an idea/pattern for easing the construction of error types.

First of all, read [this article](https://sabrinajewson.org/blog/errors).
I want to give credit to Sabrina Jewson, this is just a simple iteration on her work.  

This repo shows the same code,
but with additional methods on each modular error type to "chain" errors into specific error kinds.

## Error chaining methods

For a given Error and ErrorKind pair

```rust
pub struct DemoError {
    data: String,
    kind: DemoErrorKind
}

pub enum DemoErrorKind {
    DeeperError(DeeperError),
}
```

you can provide methods on the Error struct to ease creation:

```rust
impl DemoError {
    fn deeper_error(data: String) -> impl FnOnce(DeeperError) -> Self {
        move |error: DeeperError| {
            DemoError {
                data,
                kind: DemoErrorKind::DeeperError(error)
            }
        }
    }
}
```

they are really nice and easy to use with `Result::map_err`, watch:

```rust
pub fn demo(data: String) -> Result<(), DemoError> {
    maybe_deeper_error().map_err(DemoError::deeper_error(data))?
}
```

By creating and returning a closure, we capture the context (`data`) an error needs.

Since the returned closure matches the signature of `map_err`
(i.e. it takes an error and returns a different error)
we can just pop it straight in, lovely.

I had at one point the same basic idea but without the "return a closure" part,
but using that looks like this:

```rust
pub fn demo(data: String) -> Result<(), DemoError> {
    maybe_deeper_error(&data).map_err(|e| DemoError::deeper_error(e, data))?
}
```

In my opinion, this is noisier for no benefit.
Though admittedly the first version might cause a double take the first time it is encountered.

### Without context

Without any additional context on the error struct (e.g. `DownloadError` from lib.rs),
a closure isn't neccessary since there is no context to inject.
Just return the Error struct with the given kind directly.

```rust
impl DownloadError {
    fn read_body(error: io::Error) -> Self {
        DownloadError { kind: DownloadErrorKind::ReadBody }
    }
}
```

## Error chain start methods

Just use `new()`.

To create an error that isn't wrapping some other error,
(i.e. to start a new chain of errors)
there's no need to return a closure,
and naming the function felt hard until I realised it is just a bog-standard new constructor:

```rust

pub struct DemoError {
    data: String,
    kind: DemoErrorKind
}

pub enum DemoErrorKind {
    DeeperError(DeeperError),
}

pub fn demo() -> Result<(), DemoError> {
    let (_, _) = data
        .split_once('💥')
        .ok_or_else(DemoError::new(data, DemoErrorKind::NewChain))?
}
```

Sure, it's more verbose than `DemoError::new_chain(data)` would be,
and you have to import DemoErrorKind
but is it worth making new methods
and updating them as you change things, and add new variants?

And remembering which methods return closures and which ones return `DemoError`?
Easier with `DemoError::new()` being the only method that returns `DemoError`.

## Alternatives

I was delighted to be shown a different approach using

```rust
impl From<(ErrorKind, Context)> for Error
```

Basically, a tuple is used to conveniently insert the Context (data).

See [quinedot's comment on this IRLO post](https://internals.rust-lang.org/t/helper-for-passing-extra-context-to-errors/20259/11) discussing the idea of maybe a `Result::zip_err` method.
