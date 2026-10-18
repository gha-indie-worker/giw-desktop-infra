"""Local UDS endpoint lease reference. OS peer identity is NEVER shared-auth authority."""
from pathlib import Path
import errno,os,socket,time
class LeaseConflict(Exception):pass
def reserve(path):
    p=Path(path)
    if len(os.fsencode(p))>100:raise ValueError("unix socket path too long")
    if not p.parent.is_dir() or p.is_symlink():raise ValueError("unsafe endpoint parent")
    sock=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)
    try:
        sock.bind(str(p));sock.listen(8)
        os.chmod(p,0o600)
        return sock
    except OSError as e:
        sock.close()
        if e.errno==errno.EADDRINUSE:raise LeaseConflict("live or stale endpoint requires verified owner recovery") from e
        raise
def prune_record(timestamp_ms,now_ms,severity):
    if severity not in ("DEBUG","INFO","WARN","ERROR","FATAL"):raise ValueError("invalid severity")
    if timestamp_ms>now_ms+1000:raise ValueError("future log timestamp")
    return timestamp_ms>=now_ms-6*60*60*1000
def local_request_authorized(os_peer_verified,shared_auth_verified,scope):
    return bool(os_peer_verified and shared_auth_verified and scope in
        ("giw:read","giw:jobs","giw:admin"))
