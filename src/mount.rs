use http::server::request::{AbsolutePath};
use regex::Regex;

use iron::{Iron, Middleware, Request, Response, Alloy, Furnace};
use iron::middleware::{Status, Continue, Unwind};

#[deriving(Clone)]
pub struct Mount<F> {
    route: String,
    matches: Regex,
    iron: Iron<F>,
    terminator: Option<Status>
}

impl<F> Mount<F> {
    pub fn new(route: &str, iron: Iron<F>) -> Mount<F> {
        Mount {
            route: route.to_string(),
            iron: iron,
            matches: to_regex(route),
            terminator: Some(Unwind)
        }
    }

    pub fn non_terminal(route: &str, iron: Iron<F>) -> Mount<F> {
        Mount {
            route: route.to_string(),
            iron: iron,
            matches: to_regex(route),
            terminator: Some(Continue)
        }
    }

    pub fn filter(route: &str, iron: Iron<F>) -> Mount<F> {
        Mount {
            route: route.to_string(),
            iron: iron,
            matches: to_regex(route),
            terminator: None
        }
    }
}

fn to_regex(route: &str) -> Regex {
    Regex::new("^".to_string().append(route).as_slice()).unwrap()
}

impl<F: Furnace> Middleware for Mount<F> {
    fn enter(&mut self,
             req: &mut Request,
             res: &mut Response,
             alloy: &mut Alloy) -> Status {
        // This method is ugly, but it is hard to make it pretty
        // because we can't both borrow path from inside of request
        // while allowing furnace.forge to borrow it as mutable.

        match req.request_uri {
           AbsolutePath(ref path) => {
               // Short circuit if we don't match.
                if !self.matches.is_match(path.as_slice()) {
                    return Continue;
                }
           },
           // Short circuit if this is not an AbsolutePath.
           _ => { return Continue; }
        }

        // We are a match, so fire off to our child instance.
        match req.request_uri {
            AbsolutePath(ref mut path) => {
                *path = path.as_slice().slice_from(self.route.len()).to_string();
            },
            // Absolutely cannot happen because of our previous check,
            // but this is here just to be careful.
            _ => { return Continue; }
        } // Previous borrow of req ends here.

        // So we can borrow it again here.
        // We also take the status here, in case we are using conditional termination.
        let status = self.iron.furnace.forge(req, res, Some(alloy));

        // And repair the damage here, for future middleware
        match req.request_uri {
            AbsolutePath(ref mut path) => {
                *path = self.route.clone().append(path.as_slice());
            },
            // This really, really should never happen.
            _ => { fail!("The impossible happened."); }
        }

        // We dispatched the request, so do our final action.
        match self.terminator {
            Some(status) => status,
            None         => status
        }
    }
}

#[macro_export]
macro_rules! mount(
    ($route:expr, $midware:expr) => {
        {
            let mut subserver: ServerT = Iron::new();
            subserver.smelt($midware);
            mount::Mount::non_terminal($route, subserver)
        }
    }
)

#[macro_export]
macro_rules! mount_terminal(
    ($route:expr, $midware:expr) => {
        {
            let mut subserver: ServerT = Iron::new();
            subserver.smelt($midware);
            mount::Mount::new($route, subserver)
        }
    }
)

#[macro_export]
macro_rules! mount_filter(
    ($route:expr, $midware:expr) => {
        {
            let mut subserver: ServerT = Iron::new();
            subserver.smelt($midware);
            mount::Mount::filter($route, subserver)
        }
    }
)

