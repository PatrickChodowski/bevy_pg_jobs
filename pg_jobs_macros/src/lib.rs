#[macro_export]
macro_rules! first {
    ($var:expr, $task:ty) => {
        $var.first(Box::new($task))
    };
}

#[macro_export]
macro_rules! next {
    ($var:expr, $task:ty) => {
        $var.next(Box::new($task))
    };
}

#[macro_export]
macro_rules! next_with {
    ($var:expr, $task:ty, $next:ty) => {
        $var.next(Box::new($task)).with_next($next)
    };
}

#[macro_export]
macro_rules! first_with {
    ($var:expr, $task:ty, $next:ty) => {
        $var.first(Box::new($task)).with_next($next)
    };
}