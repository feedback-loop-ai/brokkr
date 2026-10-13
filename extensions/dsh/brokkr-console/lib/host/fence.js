// DSH serves a plugin's routes and upgrades to anyone who reaches the port
// (task 0.2's measurement): the fence is the plugin's. Every handler the
// console registers goes through one of these two wrappers, which ask DSH's
// own connection fence first and answer with its status when it refuses:
// 403 for an untrusted Host or Origin, 401 without the browser login cookie.

const REASONS = { 401: 'Unauthorized', 403: 'Forbidden' }

/** An HTTP handler that runs only for a request DSH's fence admits. */
export function fencedRoute(connection, handler) {
  return (req, res) => {
    const status = connection.requestRejection(req)
    if (status !== undefined) {
      res.statusCode = status
      res.end()
      return
    }
    return handler(req, res)
  }
}

/** An upgrade handler that runs only for a request DSH's fence admits. */
export function fencedUpgrade(connection, handler) {
  return (req, socket, head) => {
    const status = connection.requestRejection(req)
    if (status !== undefined) {
      socket.end(`HTTP/1.1 ${status} ${REASONS[status] ?? 'Refused'}\r\nconnection: close\r\n\r\n`)
      return
    }
    return handler(req, socket, head)
  }
}
