#!/usr/bin/env python3
"""Transparent live RegTest TCP proxy; delay responses to observe locked inputs.

Print the ephemeral listening port once. Terminating the process closes all
connections. All bytes are forwarded unchanged to this repo's real Ark server.
"""
import os
import socket
import socketserver
import threading
import time


class Proxy(socketserver.BaseRequestHandler):
    def handle(self):
        with socket.create_connection(("127.0.0.1", int(os.environ.get("ARK_PORT", "3635"))), timeout=10) as upstream:
            upstream.settimeout(None)

            def forward(source, destination, delay):
                try:
                    while data := source.recv(65536):
                        if delay:
                            time.sleep(0.25)
                        destination.sendall(data)
                except OSError:
                    pass
                finally:
                    try:
                        destination.shutdown(socket.SHUT_WR)
                    except OSError:
                        pass

            worker = threading.Thread(target=forward, args=(self.request, upstream, False), daemon=True)
            worker.start()
            forward(upstream, self.request, True)


class Server(socketserver.ThreadingTCPServer):
    daemon_threads = True


if __name__ == "__main__":
    with Server(("127.0.0.1", 0), Proxy) as server:
        print(server.server_address[1], flush=True)
        server.serve_forever()
