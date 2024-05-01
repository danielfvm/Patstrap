from zeroconf import Zeroconf
from pythonosc.osc_server import BlockingOSCUDPServer
from pythonosc.dispatcher import Dispatcher

import threading
import struct
import socket
import time

class Server():
    def __init__(self, window) -> None:
        self.window = window
        self.running = True
        self.reset()
        threading.Thread(target=self._connect_socket, args=()).start()
        threading.Thread(target=self._connect_osc, args=()).start()
        threading.Thread(target=self._update_loop, args=()).start()

    def reset(self):
        self.socket = None
        self.strength_right = 0
        self.strength_left = 0
        self.prev_right_value = 0
        self.prev_left_value = 0
        self.prev_right = 0
        self.prev_left = 0
        self.last_time_right = time.time()
        self.last_time_left = time.time()
        self.keepAliveTimeout = time.time()

    def set_pat(self, left: float, right: float):
        if self.socket is None:
            return

        left = int((1-left) * 15)
        right = int((1-right) * 15)
        data = (left << 4) | right

        if left == self.prev_left and right == self.prev_right:
            return

        self.prev_left = left
        self.prev_right = right

        self.socket.sendall(struct.pack('B', data))
        print(str(left) + " " + str(right) + " -> " + str(struct.pack('B', data)))

    def _get_patstrap_ip(self):
        info = None
        while not info and self.running:
            info = Zeroconf().get_service_info("_http._tcp.local.", "patstrap._http._tcp.local.")
            time.sleep(1)

        if not self.running:
            return None

        return socket.inet_ntoa(info.addresses[0])

    def _connect_socket(self):
        ip_address = self._get_patstrap_ip()

        if ip_address is None:
            return

        print("Patstrap address found: " + ip_address)

        while self.running:
            time.sleep(1)

            try:
                self.socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                self.socket.settimeout(2)
                self.socket.connect((ip_address, 8888))
                self.set_pat(0, 0)

                # Set connection status to green
                self.window.set_patstrap_status(True)

                # Wait until connection is closed
                while True:
                    result = self.socket.recv(1)
            except:
                pass

            self.window.set_patstrap_status(False)
            self.reset()
            print("Connection failed")

    def _update_loop(self):
        while self.running:
            intensity = self.window.get_intensity()
            self.strength_right = max(0, min(1, self.strength_right-0.1))
            self.strength_left = max(0, min(1, self.strength_left-0.1))

            self.set_pat(self.strength_left * intensity, self.strength_right * intensity)
            time.sleep(0.03)

            self.window.set_vrchat_status(time.time() < self.keepAliveTimeout)

    def _connect_osc(self):
        def _hit_collider_right(_, value):
            currentTime = time.time()
            if currentTime > self.last_time_right:
                self.strength_right = abs(self.prev_right_value-value)/(currentTime-self.last_time_right)
                self.prev_right_value = value
                self.last_time_right = currentTime

        def _hit_collider_left(_, value):
            currentTime = time.time()
            if currentTime > self.last_time_left:
                self.strength_left = abs(self.prev_left_value-value)/(currentTime-self.last_time_left)
                self.prev_left_value = value
                self.last_time_left = currentTime

        def _recv_packet(_, value):
            self.keepAliveTimeout = time.time() + 2

        dispatcher = Dispatcher()
        dispatcher.map("/avatar/parameters/pat_right", _hit_collider_right)
        dispatcher.map("/avatar/parameters/pat_left", _hit_collider_left)
        dispatcher.map("/avatar/parameters/*", _recv_packet)

        self.osc = BlockingOSCUDPServer(("127.0.0.1", 9001), dispatcher)
        print("OSC serving on {}".format(self.osc.server_address)) # While server is active, receive messages
        self.osc.serve_forever()

    def shutdown(self):
        self.running = False
        self.osc.shutdown()
        if self.socket is not None:
            self.socket.shutdown(2)
            self.socket.close()
