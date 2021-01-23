# Seamless

An opinionated library for creating simple JSON APIs that communicate over HTTP. The focus is on creating simple RPC style APIs over creating RESTful ones.

The main USP of this library is that it takes advantage of trait and macro magic to automatically infer the shape of the API (paths, descriptions, and the type of request and response for each route) from the Rust code, without requiring any external API definition to be created or maintained. This allows one to create (as one example) a TypeScript based API client to allow type safe communication from a browser.

API routes themselves are also slightly magical, being simple functions that can each ask for various different arguments. The route handler will only be called if it's possible to resolve each of the things it asks for based on the incoming `http::Request`.

This library is independent of any async runtime and will work nicely with any of them. Its interface is based on the `Request` and `Response` objects from the `http` package