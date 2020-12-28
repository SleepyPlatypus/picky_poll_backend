// use warp::Filter;
//
// use super::*;
// use super::operations::*;
//
// struct PickyPathsImpl<'a> {
//     post_poll: dyn Filter,
// }
//
// fn post_poll<'a>(service: &'a PollService) -> impl Filter {
//     warp::path("polls")
// }
//
// impl PickyPaths {
//     pub fn new<'a>(service: &'a PollService) -> PickyPathsImpl<'a> {
//         PickyPathsImpl{
//             post_poll: post_poll(service)
//         }
//     }
// }