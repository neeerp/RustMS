#include "../Net/Cryptography.h"
#include "../Net/FrameAssembler.h"

#include <iostream>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

namespace
{
	using Bytes = std::vector<int8_t>;

	void require(bool condition, const std::string& message)
	{
		if (!condition) throw std::runtime_error(message);
	}

	// A handshake whose send/receive IVs are `send` and `recv`.
	Bytes handshake(const int8_t (&send)[4], const int8_t (&recv)[4])
	{
		Bytes bytes(16, 0);
		for (size_t i = 0; i < 4; i++)
		{
			bytes[7 + i] = send[i];
			bytes[11 + i] = recv[i];
		}
		return bytes;
	}

	// The two ends of one direction of a connection: the server encrypts with
	// the IV the client decrypts with, as after a real handshake.
	struct Link
	{
		ms::Cryptography server;
		ms::Cryptography client;

		Link()
		{
			const int8_t client_send[4] = { 1, 2, 3, 4 };
			const int8_t client_recv[4] = { 50, 60, 70, 80 };
			const int8_t unused[4] = { 9, 9, 9, 9 };
			Bytes server_handshake = handshake(client_recv, unused);
			Bytes client_handshake = handshake(client_send, client_recv);
			server = ms::Cryptography(server_handshake.data());
			client = ms::Cryptography(client_handshake.data());
		}
	};

	Bytes plaintext(size_t length, int8_t seed)
	{
		Bytes bytes(length);
		for (size_t i = 0; i < length; i++)
			bytes[i] = static_cast<int8_t>(seed + i * 7);
		return bytes;
	}

	// Encrypt `bodies` into one wire stream, advancing the server IV per frame.
	Bytes encode(ms::Cryptography& server, const std::vector<Bytes>& bodies)
	{
		Bytes stream;
		for (const Bytes& body : bodies)
		{
			int8_t header[ms::HEADER_LENGTH];
			server.create_header(header, body.size());
			Bytes encrypted = body;
			server.encrypt(encrypted.data(), encrypted.size());
			stream.insert(stream.end(), header, header + ms::HEADER_LENGTH);
			stream.insert(stream.end(), encrypted.begin(), encrypted.end());
		}
		return stream;
	}

	struct Receiver
	{
		std::unique_ptr<ms::FrameAssembler> assembler = std::make_unique<ms::FrameAssembler>();
		std::vector<Bytes> frames;
		size_t stop_after = SIZE_MAX;

		ms::FrameAssembler::Result feed(ms::Cryptography& client, const int8_t* bytes, size_t length)
		{
			return assembler->feed(bytes, length,
				[&](const int8_t* header) { return client.check_length(header); },
				[&](int8_t* body, size_t size)
				{
					client.decrypt(body, size);
					frames.emplace_back(body, body + size);
					return frames.size() < stop_after;
				});
		}
	};

	// Every split of a stream must deliver each frame once, decrypted correctly.
	// A dropped or repeated frame desynchronizes the IV, so later plaintexts
	// would not match either.
	void every_split_point_delivers_each_frame_once()
	{
		const std::vector<Bytes> bodies = { plaintext(10, 1), plaintext(0, 2), plaintext(300, 3), plaintext(2, 4) };

		for (size_t chunk = 1; chunk <= 320; chunk++)
		{
			Link link;
			Bytes stream = encode(link.server, bodies);
			Receiver receiver;

			for (size_t offset = 0; offset < stream.size(); offset += chunk)
			{
				size_t length = std::min(chunk, stream.size() - offset);
				auto result = receiver.feed(link.client, stream.data() + offset, length);
				require(result == ms::FrameAssembler::Result::Consumed, "unexpected result for chunk " + std::to_string(chunk));
			}

			require(receiver.frames == bodies, "frames lost, repeated, or misdecrypted with chunk size " + std::to_string(chunk));
		}
	}

	// A trailing partial frame is kept until the rest of it arrives.
	void partial_trailing_frame_completes_on_the_next_read()
	{
		Link link;
		const std::vector<Bytes> bodies = { plaintext(5, 1), plaintext(40, 2) };
		Bytes stream = encode(link.server, bodies);
		Receiver receiver;

		size_t first = 4 + 5 + 4 + 10;
		receiver.feed(link.client, stream.data(), first);
		require(receiver.frames.size() == 1, "complete first frame was not dispatched");
		receiver.feed(link.client, stream.data() + first, stream.size() - first);
		require(receiver.frames == bodies, "trailing frame was not completed");
	}

	// After a handler reconnects, the rest of the old connection's bytes are dropped.
	void stopping_discards_the_rest_of_the_read()
	{
		Link link;
		Bytes stream = encode(link.server, { plaintext(8, 1), plaintext(8, 2), plaintext(8, 3) });
		Receiver receiver;
		receiver.stop_after = 1;

		auto result = receiver.feed(link.client, stream.data(), stream.size());
		require(result == ms::FrameAssembler::Result::Stopped, "stop request was ignored");
		require(receiver.frames.size() == 1, "frames were dispatched after the stop");
	}

	// A length the client cannot buffer means the stream is corrupt.
	void invalid_length_is_reported_without_dispatching()
	{
		Link link;
		// check_length XORs the two halves: 0x0000 ^ 0xFFFF is length -1.
		const int8_t header[4] = { 0, 0, -1, -1 };
		Receiver receiver;

		auto result = receiver.feed(link.client, header, sizeof(header));
		require(result == ms::FrameAssembler::Result::InvalidLength, "invalid length was accepted");
		require(receiver.frames.empty(), "a frame was dispatched for an invalid length");
	}
}

int main()
{
	try
	{
		every_split_point_delivers_each_frame_once();
		partial_trailing_frame_completes_on_the_next_read();
		stopping_discards_the_rest_of_the_read();
		invalid_length_is_reported_without_dispatching();
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
	std::cout << "frame assembler tests passed\n";
	return 0;
}
