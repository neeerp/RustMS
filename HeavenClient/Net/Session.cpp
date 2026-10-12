//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published by	//
//	the Free Software Foundation, either version 3 of the License, or			//
//	(at your option) any later version.											//
//																				//
//	This program is distributed in the hope that it will be useful,				//
//	but WITHOUT ANY WARRANTY; without even the implied warranty of				//
//	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the				//
//	GNU Affero General Public License for more details.							//
//																				//
//	You should have received a copy of the GNU Affero General Public License	//
//	along with this program.  If not, see <https://www.gnu.org/licenses/>.		//
//////////////////////////////////////////////////////////////////////////////////
#include "Session.h"

#include "../Configuration.h"

#include <vector>

namespace ms
{
	Session::Session()
	{
		connected = false;
		reconnected = false;
	}

	Session::~Session()
	{
		if (connected)
			socket.close();
	}

	bool Session::init(const char* host, const char* port)
	{
		// Connect to the server
		connected = socket.open(host, port);

		// Any partially received packet belonged to the previous connection
		assembler.reset();
		reconnected = true;

		if (connected)
		{
			// Read keys necessary for communicating with the server
			cryptography = { socket.get_buffer() };
		}

		return connected;
	}

	Error Session::init()
	{
		std::string HOST = Setting<ServerIP>::get().load();
		std::string PORT = Setting<ServerPort>::get().load();

		if (!init(HOST.c_str(), PORT.c_str()))
			return Error::CONNECTION;

		return Error::NONE;
	}

	void Session::reconnect(const char* address, const char* port)
	{
		// Close the current connection and open a new one
		bool success = socket.close();

		if (success)
			init(address, port);
		else
			connected = false;
	}

	void Session::process(const int8_t* bytes, size_t available)
	{
		auto decode_length = [&](const int8_t* header) { return cryptography.check_length(header); };

		auto on_frame = [&](int8_t* body, size_t length)
		{
			cryptography.decrypt(body, length);

			reconnected = false;

			try
			{
				packetswitch.forward(body, length);
			}
			catch (const PacketError& err)
			{
				std::cout << err.what() << std::endl;
			}

			// A handler may have switched to a new connection (for example on a server
			// redirect). The remaining bytes belong to the old connection and the socket
			// buffer was overwritten, so stop.
			return !reconnected;
		};

		if (assembler.feed(bytes, available, decode_length, on_frame) == FrameAssembler::Result::InvalidLength)
		{
			// The stream is corrupt and cannot be resynchronized. Drop the connection.
			std::cout << "Invalid packet length" << std::endl;
			connected = false;
		}
	}

	void Session::write(int8_t* packet_bytes, size_t packet_length)
	{
		if (!connected)
			return;

		int8_t header[HEADER_LENGTH];
		cryptography.create_header(header, packet_length);
		cryptography.encrypt(packet_bytes, packet_length);

		std::vector<int8_t> frame;
		frame.reserve(HEADER_LENGTH + packet_length);
		frame.insert(frame.end(), header, header + HEADER_LENGTH);
		frame.insert(frame.end(), packet_bytes, packet_bytes + packet_length);

		if (!socket.dispatch(frame.data(), frame.size()))
			connected = false;
	}

	void Session::read()
	{
		// Check if data has arrived. It does not have to be a whole packet: partial headers and
		// bodies are buffered by process() until the rest arrives.
		size_t result = socket.receive(&connected);

		if (result > 0)
		{
			// Retrieve buffer from the socket and process it
			const int8_t* bytes = socket.get_buffer();
			process(bytes, result);
		}
	}

	void Session::reconnect()
	{
		std::string HOST = Setting<ServerIP>::get().load();
		std::string PORT = Setting<ServerPort>::get().load();

		reconnect(HOST.c_str(), PORT.c_str());
	}

	bool Session::is_connected() const
	{
		return connected;
	}
}
