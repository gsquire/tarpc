// Copyright 2016 Google Inc. All Rights Reserved.
//
// Licensed under the MIT License, <LICENSE or http://opensource.org/licenses/MIT>.
// This file may not be copied, modified, or distributed except according to those terms.

/// Serde re-exports required by macros. Not for general use.
pub mod serde {
    pub use serde::{Deserialize, Deserializer, Serialize, Serializer};
    /// Deserialization re-exports required by macros. Not for general use.
    pub mod de {
        pub use serde::de::{EnumVisitor, Error, VariantVisitor, Visitor};
    }
}

// Required because if-let can't be used with irrefutable patterns, so it needs
// to be special cased.
#[doc(hidden)]
#[macro_export]
macro_rules! client_methods {
    (
        { $(#[$attr:meta])* }
        $fn_name:ident( ($($arg:ident,)*) : ($($in_:ty,)*) ) -> $out:ty
    ) => (
        $(#[$attr])*
        pub fn $fn_name(&self, $($arg: $in_),*) -> $crate::Result<$out> {
            let reply = try!((self.0).rpc(__Request::$fn_name(($($arg,)*))));
            let __Reply::$fn_name(reply) = reply;
            ::std::result::Result::Ok(reply)
        }
    );
    ($(
            { $(#[$attr:meta])* }
            $fn_name:ident( ($( $arg:ident,)*) : ($($in_:ty, )*) ) -> $out:ty
    )*) => ( $(
        $(#[$attr])*
        pub fn $fn_name(&self, $($arg: $in_),*) -> $crate::Result<$out> {
            let reply = try!((self.0).rpc(__Request::$fn_name(($($arg,)*))));
            if let __Reply::$fn_name(reply) = reply {
                ::std::result::Result::Ok(reply)
            } else {
                panic!("Incorrect reply variant returned from protocol::Clientrpc; expected `{}`, but got {:?}", stringify!($fn_name), reply);
            }
        }
    )*);
}

// Required because if-let can't be used with irrefutable patterns, so it needs
// to be special cased.
#[doc(hidden)]
#[macro_export]
macro_rules! async_client_methods {
    (
        { $(#[$attr:meta])* }
        $fn_name:ident( ($( $arg:ident, )*) : ($( $in_:ty, )*) ) -> $out:ty
    ) => (
        $(#[$attr])*
        pub fn $fn_name(&self, $($arg: $in_),*) -> Future<$out> {
            fn mapper(reply: __Reply) -> $out {
                let __Reply::$fn_name(reply) = reply;
                reply
            }
            let reply = (self.0).rpc_async(__Request::$fn_name(($($arg,)*)));
            Future {
                future: reply,
                mapper: mapper,
            }
        }
    );
    ($(
            { $(#[$attr:meta])* }
            $fn_name:ident( ($( $arg:ident, )*) : ($( $in_:ty, )*) ) -> $out:ty
    )*) => ( $(
        $(#[$attr])*
        pub fn $fn_name(&self, $($arg: $in_),*) -> Future<$out> {
            fn mapper(reply: __Reply) -> $out {
                if let __Reply::$fn_name(reply) = reply {
                    reply
                } else {
                    panic!("Incorrect reply variant returned from protocol::Clientrpc; expected `{}`, but got {:?}", stringify!($fn_name), reply);
                }
            }
            let reply = (self.0).rpc_async(__Request::$fn_name(($($arg,)*)));
            Future {
                future: reply,
                mapper: mapper,
            }
        }
    )*);
}

#[doc(hidden)]
#[macro_export]
macro_rules! impl_serialize {
    ($impler:ident, $(@($name:ident $n:expr))* -- #($_n:expr) ) => (
        impl $crate::macros::serde::Serialize for $impler {
            #[inline]
            fn serialize<S>(&self, serializer: &mut S) -> ::std::result::Result<(), S::Error>
                where S: $crate::macros::serde::Serializer
            {
                match *self {
                    $(
                        $impler::$name(ref field) =>
                            $crate::macros::serde::Serializer::visit_newtype_variant(
                                serializer,
                                stringify!($impler),
                                $n,
                                stringify!($name),
                                field,
                            )
                    ),*
                }
            }
        }
    );
    // All args are wrapped in a tuple so we can use the newtype variant for each one.
    ($impler:ident, $(@$finished:tt)* -- #($n:expr) $name:ident($field:ty) $($req:tt)*) => (
        impl_serialize!($impler, $(@$finished)* @($name $n) -- #($n + 1) $($req)*);
    );
    // Entry
    ($impler:ident, $($started:tt)*) => (impl_serialize!($impler, -- #(0) $($started)*););
}

#[doc(hidden)]
#[macro_export]
macro_rules! impl_deserialize {
    ($impler:ident, $(@($name:ident $n:expr))* -- #($_n:expr) ) => (
        impl $crate::macros::serde::Deserialize for $impler {
            #[inline]
            fn deserialize<D>(deserializer: &mut D)
                -> ::std::result::Result<$impler, D::Error>
                where D: $crate::macros::serde::Deserializer
            {
                #[allow(non_camel_case_types)]
                enum __Field {
                    $($name),*
                }
                impl $crate::macros::serde::Deserialize for __Field {
                    #[inline]
                    fn deserialize<D>(deserializer: &mut D)
                        -> ::std::result::Result<__Field, D::Error>
                        where D: $crate::macros::serde::Deserializer
                    {
                        struct __FieldVisitor;
                        impl $crate::macros::serde::de::Visitor for __FieldVisitor {
                            type Value = __Field;

                            #[inline]
                            fn visit_usize<E>(&mut self, value: usize)
                                -> ::std::result::Result<__Field, E>
                                where E: $crate::macros::serde::de::Error,
                            {
                                $(
                                    if value == $n {
                                        return ::std::result::Result::Ok(__Field::$name);
                                    }
                                )*
                                return ::std::result::Result::Err(
                                    $crate::macros::serde::de::Error::syntax("expected a field")
                                );
                            }
                        }
                        deserializer.visit_struct_field(__FieldVisitor)
                    }
                }

                struct __Visitor;
                impl $crate::macros::serde::de::EnumVisitor for __Visitor {
                    type Value = $impler;

                    #[inline]
                    fn visit<__V>(&mut self, mut visitor: __V)
                        -> ::std::result::Result<$impler, __V::Error>
                        where __V: $crate::macros::serde::de::VariantVisitor
                    {
                        match try!(visitor.visit_variant()) {
                            $(
                                __Field::$name => {
                                    let val = try!(visitor.visit_newtype());
                                    Ok($impler::$name(val))
                                }
                            ),*
                        }
                    }
                }
                const VARIANTS: &'static [&'static str] = &[
                    $(
                        stringify!($name)
                    ),*
                ];
                deserializer.visit_enum(stringify!($impler), VARIANTS, __Visitor)
            }
        }
    );
    // All args are wrapped in a tuple so we can use the newtype variant for each one.
    ($impler:ident, $(@$finished:tt)* -- #($n:expr) $name:ident($field:ty) $($req:tt)*) => (
        impl_deserialize!($impler, $(@$finished)* @($name $n) -- #($n + 1) $($req)*);
    );
    // Entry
    ($impler:ident, $($started:tt)*) => (impl_deserialize!($impler, -- #(0) $($started)*););
}

/// The main macro that creates RPC services.
///
/// Rpc methods are specified, mirroring trait syntax:
///
/// ```
/// # #[macro_use] extern crate tarpc;
/// # fn main() {}
/// # service! {
/// #[doc="Say hello"]
/// rpc hello(name: String) -> String;
/// # }
/// ```
///
/// Attributes can be attached to each rpc. These attributes
/// will then be attached to the generated `Service` trait's
/// corresponding method, as well as to the `Client` stub's rpcs methods.
///
/// The following items are expanded in the enclosing module:
///
/// * `Service` -- the trait defining the RPC service
/// * `Client` -- a client that makes synchronous requests to the RPC server
/// * `AsyncClient` -- a client that makes asynchronous requests to the RPC server
/// * `Future` -- a handle for asynchronously retrieving the result of an RPC
/// * `serve` -- the function that starts the RPC server
///
/// **Warning**: In addition to the above items, there are a few expanded items that
/// are considered implementation details. As with the above items, shadowing
/// these item names in the enclosing module is likely to break things in confusing
/// ways:
///
/// * `__Server` -- an implementation detail
/// * `__Request` -- an implementation detail
/// * `__Reply` -- an implementation detail
#[macro_export]
macro_rules! service {
    (
        $( $tokens:tt )*
    ) => {
        service_inner! {{
            $( $tokens )*
        }}
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! service_inner {
    // Pattern for when the next rpc has an implicit unit return type
    (
        {
            $(#[$attr:meta])*
            rpc $fn_name:ident( $( $arg:ident : $in_:ty ),* );

            $( $unexpanded:tt )*
        }
        $( $expanded:tt )*
    ) => {
        service_inner! {
            { $( $unexpanded )* }

            $( $expanded )*

            $(#[$attr])*
            rpc $fn_name( $( $arg : $in_ ),* ) -> ();
        }
    };
    // Pattern for when the next rpc has an explicit return type
    (
        {
            $(#[$attr:meta])*
            rpc $fn_name:ident( $( $arg:ident : $in_:ty ),* ) -> $out:ty;

            $( $unexpanded:tt )*
        }
        $( $expanded:tt )*
    ) => {
        service_inner! {
            { $( $unexpanded )* }

            $( $expanded )*

            $(#[$attr])*
            rpc $fn_name( $( $arg : $in_ ),* ) -> $out;
        }
    };
    // Pattern when all return types have been expanded
    (
        { } // none left to expand
        $(
            $(#[$attr:meta])*
            rpc $fn_name:ident ( $( $arg:ident : $in_:ty ),* ) -> $out:ty;
        )*
    ) => {
        #[doc="Defines the RPC service"]
        pub trait Service: Send + Sync {
            $(
                $(#[$attr])*
                fn $fn_name(&self, $($arg:$in_),*) -> $out;
            )*
        }

        impl<P, S> Service for P
            where P: Send + Sync + ::std::ops::Deref<Target=S>,
                  S: Service
        {
            $(
                $(#[$attr])*
                fn $fn_name(&self, $($arg:$in_),*) -> $out {
                    Service::$fn_name(&**self, $($arg),*)
                }
            )*
        }

        #[allow(non_camel_case_types)]
        #[derive(Debug)]
        enum __Request {
            $(
                $fn_name(( $($in_,)* ))
            ),*
        }

        impl_serialize!(__Request, $($fn_name(($($in_),*)))*);
        impl_deserialize!(__Request, $($fn_name(($($in_),*)))*);

        #[allow(non_camel_case_types)]
        #[derive(Debug)]
        enum __Reply {
            $(
                $fn_name($out),
            )*
        }

        impl_serialize!(__Reply, $($fn_name($out))*);
        impl_deserialize!(__Reply, $($fn_name($out))*);

        /// An asynchronous RPC call
        pub struct Future<T> {
            future: $crate::protocol::Future<__Reply>,
            mapper: fn(__Reply) -> T,
        }

        impl<T> Future<T> {
            /// Block until the result of the RPC call is available
            pub fn get(self) -> $crate::Result<T> {
                self.future.get().map(self.mapper)
            }
        }

        #[doc="The client stub that makes RPC calls to the server."]
        pub struct Client($crate::protocol::Client<__Request, __Reply>);

        impl Client {
            #[doc="Create a new client that connects to the given address."]
            pub fn new<A>(addr: A, timeout: ::std::option::Option<::std::time::Duration>)
                -> $crate::Result<Self>
                where A: ::std::net::ToSocketAddrs,
            {
                let inner = try!($crate::protocol::Client::new(addr, timeout));
                ::std::result::Result::Ok(Client(inner))
            }

            client_methods!(
                $(
                    { $(#[$attr])* }
                    $fn_name(($($arg,)*) : ($($in_,)*)) -> $out
                )*
            );

            #[doc="Attempt to clone the client object. This might fail if the underlying TcpStream \
                   clone fails."]
            pub fn try_clone(&self) -> ::std::io::Result<Self> {
                ::std::result::Result::Ok(Client(try!(self.0.try_clone())))
            }
        }

        #[doc="The client stub that makes asynchronous RPC calls to the server."]
        pub struct AsyncClient($crate::protocol::Client<__Request, __Reply>);

        impl AsyncClient {
            #[doc="Create a new asynchronous client that connects to the given address."]
            pub fn new<A>(addr: A, timeout: ::std::option::Option<::std::time::Duration>)
                -> $crate::Result<Self>
                where A: ::std::net::ToSocketAddrs,
            {
                let inner = try!($crate::protocol::Client::new(addr, timeout));
                ::std::result::Result::Ok(AsyncClient(inner))
            }

            async_client_methods!(
                $(
                    { $(#[$attr])* }
                    $fn_name(($($arg,)*): ($($in_,)*)) -> $out
                )*
            );

            #[doc="Attempt to clone the client object. This might fail if the underlying TcpStream \
                   clone fails."]
            pub fn try_clone(&self) -> ::std::io::Result<Self> {
                ::std::result::Result::Ok(AsyncClient(try!(self.0.try_clone())))
            }
        }

        struct __Server<S: 'static + Service>(S);

        impl<S> $crate::protocol::Serve for __Server<S>
            where S: 'static + Service
        {
            type Request = __Request;
            type Reply = __Reply;
            fn serve(&self, request: __Request) -> __Reply {
                match request {
                    $(
                        __Request::$fn_name(( $($arg,)* )) =>
                            __Reply::$fn_name((self.0).$fn_name($($arg),*)),
                    )*
                }
            }
        }

        #[doc="Start a running service."]
        pub fn serve<A, S>(addr: A,
                           service: S,
                           read_timeout: ::std::option::Option<::std::time::Duration>)
            -> $crate::Result<$crate::protocol::ServeHandle>
            where A: ::std::net::ToSocketAddrs,
                  S: 'static + Service
        {
            let server = ::std::sync::Arc::new(__Server(service));
            ::std::result::Result::Ok(try!($crate::protocol::serve_async(addr, server, read_timeout)))
        }
    }
}

#[allow(dead_code)] // because we're just testing that the macro expansion compiles
#[cfg(test)]
mod syntax_test {
    // Tests a service definition with a fn that takes no args
    mod qux {
        service! {
            rpc hello() -> String;
        }
    }
    // Tests a service definition with an attribute.
    mod bar {
        service! {
            #[doc="Hello bob"]
            rpc baz(s: String) -> String;
        }
    }

    // Tests a service with implicit return types.
    mod no_return {
        service! {
            rpc ack();
            rpc apply(foo: String) -> i32;
            rpc bi_consume(bar: String, baz: u64);
            rpc bi_fn(bar: String, baz: u64) -> String;
        }
    }
}

#[cfg(test)]
mod functional_test {
    extern crate env_logger;
    use std::time::Duration;

    fn test_timeout() -> Option<Duration> {
        Some(Duration::from_secs(5))
    }

    service! {
        rpc add(x: i32, y: i32) -> i32;
        rpc hey(name: String) -> String;
    }

    struct Server;

    impl Service for Server {
        fn add(&self, x: i32, y: i32) -> i32 {
            x + y
        }
        fn hey(&self, name: String) -> String {
            format!("Hey, {}.", name)
        }
    }

    #[test]
    fn simple() {
        let _ = env_logger::init();
        let handle = serve("localhost:0", Server, test_timeout()).unwrap();
        let client = Client::new(handle.local_addr(), None).unwrap();
        assert_eq!(3, client.add(1, 2).unwrap());
        assert_eq!("Hey, Tim.", client.hey("Tim".into()).unwrap());
        drop(client);
        handle.shutdown();
    }

    #[test]
    fn simple_async() {
        let _ = env_logger::init();
        let handle = serve("localhost:0", Server, test_timeout()).unwrap();
        let client = AsyncClient::new(handle.local_addr(), None).unwrap();
        assert_eq!(3, client.add(1, 2).get().unwrap());
        assert_eq!("Hey, Adam.", client.hey("Adam".into()).get().unwrap());
        drop(client);
        handle.shutdown();
    }

    #[test]
    fn try_clone() {
        let handle = serve("localhost:0", Server, test_timeout()).unwrap();
        let client1 = Client::new(handle.local_addr(), None).unwrap();
        let client2 = client1.try_clone().unwrap();
        assert_eq!(3, client1.add(1, 2).unwrap());
        assert_eq!(3, client2.add(1, 2).unwrap());
    }

    #[test]
    fn async_try_clone() {
        let handle = serve("localhost:0", Server, test_timeout()).unwrap();
        let client1 = AsyncClient::new(handle.local_addr(), None).unwrap();
        let client2 = client1.try_clone().unwrap();
        assert_eq!(3, client1.add(1, 2).get().unwrap());
        assert_eq!(3, client2.add(1, 2).get().unwrap());
    }

    // Tests that a server can be wrapped in an Arc; no need to run, just compile
    #[allow(dead_code)]
    fn serve_arc_server() {
        let _ = serve("localhost:0", ::std::sync::Arc::new(Server), None);
    }

    #[test]
    fn serde() {
        use bincode;
        let _ = env_logger::init();

        let request = __Request::add((1, 2));
        let ser = bincode::serde::serialize(&request, bincode::SizeLimit::Infinite).unwrap();
        let de = bincode::serde::deserialize(&ser).unwrap();
        if let __Request::add((1, 2)) = de {
            // success
        } else {
            panic!("Expected __Request::add, got {:?}", de);
        }
    }
}
