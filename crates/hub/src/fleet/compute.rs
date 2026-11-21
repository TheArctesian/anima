//! Model compute on the operator's nodes: the hardware facts, the model library, the placement
//! of a model on a node and the device policy of each node (compute plan, Section 13). Built:
//! the pure placement, and the device policy with its GraphQL part.

pub mod graphql;
pub mod placement;
pub mod policy;
