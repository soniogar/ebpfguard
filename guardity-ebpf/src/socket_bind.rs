use core::cmp;

use aya_bpf::{cty::c_long, programs::LsmContext, BpfContext};
use guardity_common::{AlertSocketBind, MAX_PORTS};

use crate::{
    binprm::current_binprm_inode,
    consts::{AF_INET, INODE_WILDCARD},
    maps::{ALERT_SOCKET_BIND, ALLOWED_SOCKET_BIND, DENIED_SOCKET_BIND},
    vmlinux::{sockaddr, sockaddr_in},
};

/// Inspects the context of `socket_bind` LSM hook and decides whether to allow
/// or deny the bind operation based on the state of the `ALLOWED_SOCKET_BIND`
/// and `DENIED_SOCKET_BIND` maps.
///
/// If denied, the operation is logged to the `ALERT_SOCKET_BIND` map.
///
/// # Example
///
/// ```rust
/// use aya_bpf::{macros::lsm, programs::LsmContext};
/// use guardity_ebpf::socket_bind;
///
/// #[lsm(name = "my_program")]
/// pub fn my_program(ctx: LsmContext) -> i32 {
///     match socket_bind::socket_bind(ctx) {
///         Ok(ret) => ret,
///         Err(_) => 0,
///     }
/// }
/// ```
#[inline(always)]
pub fn socket_bind(ctx: LsmContext) -> Result<i32, c_long> {
    let sockaddr: *const sockaddr = unsafe { ctx.arg(1) };

    if unsafe { (*sockaddr).sa_family } != AF_INET {
        return Ok(0);
    }

    let sockaddr_in: *const sockaddr_in = sockaddr as *const sockaddr_in;
    let port = u16::from_be(unsafe { (*sockaddr_in).sin_port });

    let binprm_inode = current_binprm_inode();

    if let Some(ports) = unsafe { ALLOWED_SOCKET_BIND.get(&INODE_WILDCARD) } {
        if ports.all {
            if let Some(ports) = unsafe { DENIED_SOCKET_BIND.get(&INODE_WILDCARD) } {
                if ports.all {
                    ALERT_SOCKET_BIND.output(
                        &ctx,
                        &AlertSocketBind::new(ctx.pid(), binprm_inode, port),
                        0,
                    );
                    return Ok(-1);
                }
                let len = cmp::min(ports.len, MAX_PORTS);
                if ports.ports[..len].contains(&port) {
                    ALERT_SOCKET_BIND.output(
                        &ctx,
                        &AlertSocketBind::new(ctx.pid(), binprm_inode, port),
                        0,
                    );
                    return Ok(-1);
                }
            }

            if let Some(ports) = unsafe { DENIED_SOCKET_BIND.get(&binprm_inode) } {
                if ports.all {
                    ALERT_SOCKET_BIND.output(
                        &ctx,
                        &AlertSocketBind::new(ctx.pid(), binprm_inode, port),
                        0,
                    );
                    return Ok(-1);
                }
                let len = cmp::min(ports.len, MAX_PORTS);
                if ports.ports[..len].contains(&port) {
                    ALERT_SOCKET_BIND.output(
                        &ctx,
                        &AlertSocketBind::new(ctx.pid(), binprm_inode, port),
                        0,
                    );
                    return Ok(-1);
                }
            }
        } else {
            let len = cmp::min(ports.len, MAX_PORTS);
            if ports.ports[..len].contains(&port) {
                return Ok(0);
            }
        }
    }

    if let Some(ports) = unsafe { DENIED_SOCKET_BIND.get(&INODE_WILDCARD) } {
        if ports.all {
            if let Some(ports) = unsafe { ALLOWED_SOCKET_BIND.get(&INODE_WILDCARD) } {
                if ports.all {
                    return Ok(0);
                }
                let len = cmp::min(ports.len, MAX_PORTS);
                if ports.ports[..len].contains(&port) {
                    return Ok(0);
                }
            }

            if let Some(ports) = unsafe { ALLOWED_SOCKET_BIND.get(&binprm_inode) } {
                if ports.all {
                    return Ok(0);
                }
                let len = cmp::min(ports.len, MAX_PORTS);
                if ports.ports[..len].contains(&port) {
                    return Ok(0);
                }
            }

            ALERT_SOCKET_BIND.output(
                &ctx,
                &AlertSocketBind::new(ctx.pid(), binprm_inode, port),
                0,
            );
            return Ok(-1);
        } else {
            let len = cmp::min(ports.len, MAX_PORTS);
            if ports.ports[..len].contains(&port) {
                ALERT_SOCKET_BIND.output(
                    &ctx,
                    &AlertSocketBind::new(ctx.pid(), binprm_inode, port),
                    0,
                );
                return Ok(-1);
            }
        }
    }

    Ok(0)
}