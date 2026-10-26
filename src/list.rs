#![allow(unused)]

/*
* why a single node must point to itself ?
* The circular-list says that repeatedly following next must eventually return to the starting
* node. Since A  is the only node available, A's sucessor must be A.
* therefore:
*          A.next = A
* reciprocal-link invariant. If A's successor is A, then that successor's predecessor must be A:
*          A.next = A ==> A.next.prev = A
* because A.next is A itself, this simplifies to:
*          A.prev = A
* so both link must point to A:
*          A.next = A.prev = A
* Insertion:
* Insertion doesn't create a linked list from scratch. It changes a small set of relationships in an existing
* graph while preserving the invariants.
*
* suppose we have three nodes, A, B, and C.
* [A prev: C next: B] -> [B prev: A next: C] -> C [prev: B next: A]
* A -> B -> C -> A
*
* we want to insert a new node X between A and B
* we are replacing adjacency relationship A -> B with two relationships:
* - A -> X
* - X -> B
* but because this is a doubly linked list we must also preserve backward relationship.
*
* the four relationship must change
* before insertion:
*         A.next = B, B.prev = A
* after insertion:
*         A.next = X, X.prev = A
*         X.next = B, B.prev = X

* A [next: X prev: C] -> X [next: B prev: A] -> B [next: C prev: X]
* -> Continue B -> C -> A to close the cycle
*
* circular boundary:
* head.prev = tail <==> tail.next = head
*
* basic relationship:
* head -> A -> B -> C and C -> B -> A
 A.prev = C
 A.next = B
 B.prev = A
 B.next = C
 C.prev = B
 C.next = A

 note:
 here Head is actual ListHead its not textbook style where Head just references to first Node
 Head.next = first real node
 Head.prev = Tail last real node

 its like an reaction:
X.next = head          <==>  head.prev     =  X
X.prev = old_tail.prev <==>  old_tail.next =  X

if you update prev then you must update next, which also implies if you update next you mut also update prev
*/

pub struct ListHead {
    prev: *mut ListHead,
    next: *mut ListHead,
}

unsafe fn test() {
    let mut head = ListHead::new();
    ListHead::init(core::ptr::addr_of_mut!(head));

    let mut list = ListHead::new();
    ListHead::init(core::ptr::addr_of_mut!(list));
    list.add(core::ptr::addr_of_mut!(head));
}

impl ListHead {
    pub const fn new() -> Self {
        Self {
            prev: core::ptr::null_mut(),
            next: core::ptr::null_mut(),
        }
    }

    pub unsafe fn init(node: *mut ListHead) {
        (*node).prev = node;
        (*node).next = node;
    }

    // &self -> represents new

    pub const fn is_empty(&self, head: *const Self) -> bool {
        todo!()
    }

    /// inserts new between two existing nodes: prev and next
    /// self: new node being inserted
    /// head: the node that should come immediately before self
    /// next: the node that should come immediately after self
    unsafe fn __add(&mut self, prev: *mut Self, next: *mut Self) {
        // insert self between prev and next
        // the core rule: if A.next = B then B.prev = A
        // A.next = B <==> B.prev = A
        // to insert a node between A and B:
        // X.prev = A, X.next = B
        // A.next = X, B.prev = X
        // prev: A, next: B, self: X
        // currently:
        // A -> B and B -> A
        // we want:
        // A -> X -> B and B -> X -> A
        self.prev = prev;
        self.next = next;
        (*prev).next = self as *mut ListHead;
        (*next).prev = self as *mut ListHead;
    }

    /// inserts new immediately after head, at the beginning of the list
    /// self: represents new
    /// head: represents the list head
    pub unsafe fn add(&mut self, head: *mut Self) {
        // now how can i call __add so it inserts at start of the node
        // let head = prev, next = head.next
        // assume:
        // head -> A -> B -> C and C -> B -> A
        // A.prev = C
        // A.next = B
        // B.prev = A
        // B.next = C
        // C.prev = B
        // C.next = A

        // so to insert X at head we want:
        // X.next    = head       X.prev = head.prev
        // head.prev = X
        //
        // head never changes:
        // X.prev         = head,       X.next    = head.next
        // head.next.prev = X,          head.next = X

        self.__add(head, (*head).next);
    }

    pub unsafe fn add_tail(&mut self, head: *mut Self) {
        // X.next = head          <==>  head.prev     =  X
        // X.prev = old_tail      <==>  old_tail.next =  X
        self.__add((*head).prev, head);
    }

    pub fn add_before(&mut self, entry: *mut Self) {
        // insert node before entry
    }

    pub fn add_after(&mut self, entry: *mut Self) {
        // insert node after entry
    }

    pub fn del(&mut self) {
        // delete a node
    }
}
