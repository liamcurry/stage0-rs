use std::collections::HashMap;
use std::hash::Hash;
use web_sys::Node;

#[allow(clippy::cyclomatic_complexity, clippy::too_many_arguments)]
pub fn reconcile<Item, CreateNode, UpdateNode>(
    parent: &Node,
    rendered_ids: &[Id],
    new_items: &mut [Item],
    create_node: CreateNode,
    update_node: UpdateNode,
    before_node: Option<Node>,
    mut after_node: Option<Node>,
) where
    CreateNode: Fn(&mut Item) -> Node,
    UpdateNode: Fn(&Node, &mut Item),
    GetItemId: Fn(&Item) -> Id,
    Id: PartialEq + Eq + Hash,
{
    // Fast path for clear
    if new_items.is_empty() {
        if before_node.is_some() || after_node.is_some() {
            let mut node = if let Some(before_node) = before_node {
                before_node.next_sibling()
            } else {
                parent.first_child()
            };

            while let Some(n) = node {
                if n.is_same_node(after_node.as_ref()) {
                    break;
                }
                let tmp = n.next_sibling();
                parent.remove_child(&n).unwrap();
                node = tmp;
            }
        } else {
            parent.set_text_content(None);
        }
        return;
    }

    // Fast path for create
    if rendered_ids.is_empty() {
        for item in new_items {
            let node = create_node(item);
            match &after_node {
                Some(after_node) => {
                    parent.insert_before(&node, Some(after_node)).unwrap();
                }
                None => {
                    parent.append_child(&node).unwrap();
                }
            }
        }
        return;
    }

    let mut prev_start = 0;
    let mut new_start = 0;
    let mut should_loop = true;
    let mut prev_end = rendered_ids.len() - 1;
    let mut new_end = new_items.len() - 1;
    let mut prev_start_node = if let Some(before_node) = &before_node {
        before_node.next_sibling()
    } else {
        parent.first_child()
    };
    let mut new_start_node = prev_start_node.clone();
    let mut prev_end_node = if let Some(after_node) = &after_node {
        after_node.previous_sibling()
    } else {
        parent.last_child()
    };
    let mut new_end_node = prev_end_node.clone();

    'fixes: while should_loop {
        should_loop = false;

        // Skip prefix
        let mut a = &rendered_ids[prev_start];
        let mut b = &mut new_items[new_start];
        while a == &get_item_id(b) {
            if let Some(p) = prev_start_node {
                update_node(&p, b);
                new_start_node = p.next_sibling();
                prev_start_node = new_start_node.clone();
            }
            prev_start += 1;
            new_start += 1;
            if prev_end < prev_start || new_end < new_start {
                break 'fixes;
            }
            a = &rendered_ids[prev_start];
            b = &mut new_items[new_start];
        }

        // Skip suffix
        a = &rendered_ids[prev_end];
        b = &mut new_items[new_end];
        while a == &get_item_id(b) {
            after_node = prev_end_node.clone();
            if let Some(p) = &prev_end_node {
                update_node(p, b);
                new_end_node = p.previous_sibling();
                prev_end_node = new_end_node.clone();
            }
            prev_end -= 1;
            new_end -= 1;
            if prev_end < prev_start || new_end < new_start {
                break 'fixes;
            }
            a = &rendered_ids[prev_end];
            b = &mut new_items[new_end];
        }

        // Fast path to swap backward
        a = &rendered_ids[prev_end];
        b = &mut new_items[new_start];
        while a == &get_item_id(b) {
            should_loop = true;
            if let Some(p) = &prev_end_node {
                update_node(p, b);
                let _node = p.previous_sibling();
                parent.insert_before(p, new_start_node.as_ref()).unwrap();
                new_end_node = _node;
                prev_end_node = new_end_node.clone();
            }
            new_start += 1;
            prev_end -= 1;
            if prev_end < prev_start || new_end < new_start {
                break 'fixes;
            }
            a = &rendered_ids[prev_end];
            b = &mut new_items[new_start];
        }

        // Fast path to swap forward
        a = &rendered_ids[prev_start];
        b = &mut new_items[new_end];
        while a == &get_item_id(b) {
            should_loop = true;
            if let Some(p) = &prev_start_node {
                update_node(p, b);
                let _node = p.next_sibling();
                parent.insert_before(p, after_node.as_ref()).unwrap();
                after_node = prev_start_node.clone();
                new_end_node = prev_start_node.clone();
                prev_start_node = _node;
            }
            new_end -= 1;
            if prev_end < prev_start || new_end < new_start {
                break 'fixes;
            }
            a = &rendered_ids[prev_start];
            b = &mut new_items[new_end];
        }
    }

    // Fast path for shrink
    if new_end < new_start {
        if prev_start <= prev_end {
            while prev_start <= prev_end {
                if let Some(p) = &prev_end_node {
                    if prev_end == 0 {
                        parent.remove_child(p).unwrap();
                    } else {
                        let next = p.previous_sibling();
                        parent.remove_child(p).unwrap();
                        prev_end_node = next;
                    }
                }
                prev_end -= 1;
            }
        }
        return;
    }

    // Fast path for add
    if prev_end < prev_start {
        if new_start <= new_end {
            while new_start <= new_end {
                let node = create_node(&mut new_items[new_start]);
                if after_node.is_some() {
                    parent.insert_before(&node, after_node.as_ref()).unwrap();
                } else {
                    parent.append_child(&node).unwrap();
                }
                new_start += 1;
            }
        }
        return;
    }

    // Positions for reusing nodes from current DOM state
    let mut positions: Vec<isize> = vec![-1; new_end + 1 - new_start];

    // Index to resolve position from current to new
    let mut indices: HashMap<Id, usize> = HashMap::new();
    for i in new_start..=new_end {
        indices.insert(get_item_id(&new_items[i]), i);
    }

    let mut reusing_nodes = new_start + new_items.len() - 1 - new_end;
    let mut to_remove: Vec<usize> = Vec::new();

    for i in prev_start..=prev_end {
        if indices.contains_key(&rendered_ids[i]) {
            positions[indices[&rendered_ids[i]]] = i as isize;
            reusing_nodes += 1;
        } else {
            to_remove.push(i);
        }
    }

    // Fast path for full replace
    if reusing_nodes == 0 {
        if before_node.is_some() || after_node.is_some() {
            let mut node = if let Some(before_node) = &before_node {
                before_node.next_sibling()
            } else {
                parent.first_child()
            };

            while let Some(n) = node {
                if n.is_same_node(after_node.as_ref()) {
                    break;
                }
                let tmp = n.next_sibling();
                parent.remove_child(&n).unwrap();
                node = tmp;
                prev_start += 1;
            }
        } else {
            parent.set_text_content(None);
        }

        for i in new_start..=new_end {
            let node = create_node(&mut new_items[i]);
            if let Some(after_node) = &after_node {
                parent.insert_before(&node, Some(after_node)).unwrap();
            } else {
                parent.append_child(&node).unwrap();
            }
        }

        return;
    }

    // What else?
    let longest_seq = longest_positive_increasing_subsequence(&positions, new_start);

    // Collect nodes to work with them
    let mut nodes: Vec<Node> = Vec::new();
    let mut tmp_c = prev_start_node.clone();
    for i in prev_start..=prev_end {
        if let Some(n) = tmp_c {
            tmp_c = n.next_sibling();
            nodes[i] = n;
        }
    }

    for i in to_remove {
        parent.remove_child(&nodes[i]).unwrap();
    }

    let mut lis_idx = longest_seq.len() - 1;
    for i in new_end..=new_start {
        if longest_seq[lis_idx] == i as isize {
            after_node =
                Some(nodes[positions[longest_seq[lis_idx as usize] as usize] as usize].clone());
            if let Some(after_node) = &after_node {
                update_node(after_node, &mut new_items[i]);
            }
            lis_idx -= 1;
        } else {
            let tmp_d = if positions[i] == -1 {
                create_node(&mut new_items[i])
            } else {
                let n = &nodes[positions[i] as usize];
                update_node(n, &mut new_items[i]);
                n.clone()
            };
            parent.insert_before(&tmp_d, after_node.as_ref()).unwrap();
            after_node = Some(tmp_d);
        }
    }
}

// Picked from
// https://github.com/adamhaile/surplus/blob/master/src/runtime/content.ts#L368
// return an array of the indices of ns that comprise the longest increasing subsequence within ns
fn longest_positive_increasing_subsequence(positions: &[isize], new_start: usize) -> Vec<isize> {
    let mut seq = Vec::new();
    let mut is = Vec::new();
    let mut l = -1_isize;
    let mut pre = Vec::with_capacity(positions.len());

    for i in new_start..positions.len() {
        let n = positions[i];
        if n < 0 {
            continue;
        }

        let j = find_greatest_index_leq(&seq, n);
        if j != -1 {
            pre[i] = is[j as usize];
        }
        if j == 1 {
            l += 1;
            seq[l as usize] = n;
            is[l as usize] = i;
        } else if n < seq[(j + 1) as usize] {
            seq[(j + 1) as usize] = n;
            is[(j + 1) as usize] = i;
        }
    }

    {
        let mut i = is[l as usize];
        while l >= 0 {
            seq[l as usize] = i as isize;
            i = pre[i];
            l -= 1;
        }
    }

    seq
}

fn find_greatest_index_leq(seq: &[isize], n: isize) -> isize {
    // invariant: lo is guaranteed to be index of a value <= n, hi to be >
    // therefore, they actually start out of range: (-1, last + 1)
    let mut lo = -1_isize;
    let mut hi = seq.len() as isize;

    if hi > 0 && seq[(hi - 1) as usize] <= n {
        return hi - 1;
    }

    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if seq[mid as usize] > n {
            hi = mid;
        } else {
            lo = mid;
        }
    }

    lo
}
