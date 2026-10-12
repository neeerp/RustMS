#pragma once

#include "NetConstants.h"

#include <cstddef>
#include <cstdint>
#include <cstring>

namespace ms
{
	// Reassembles length-prefixed frames from a TCP byte stream. A read may end
	// in the middle of a header or a body, or contain several frames; partial
	// state is kept until the next read, and no byte is dropped or reused.
	class FrameAssembler
	{
	public:
		enum class Result
		{
			// Every byte was consumed (any partial frame is kept for later).
			Consumed,
			// on_frame asked to stop; the remaining bytes were discarded.
			Stopped,
			// A header announced an impossible length; the stream is unusable.
			InvalidLength
		};

		FrameAssembler()
		{
			reset();
		}

		// Forget any partial frame (for example after reconnecting).
		void reset()
		{
			headerpos = 0;
			length = 0;
			pos = 0;
		}

		// Consume `available` bytes. `decode_length(header)` returns the body
		// length announced by a complete header. `on_frame(body, length)` is
		// called once per complete frame and returns false to stop processing
		// the rest of this read (the bytes then belong to a closed connection).
		template<class DecodeLength, class OnFrame>
		Result feed(const int8_t* bytes, size_t available, DecodeLength decode_length, OnFrame on_frame)
		{
			while (available > 0)
			{
				if (headerpos < HEADER_LENGTH)
				{
					size_t tocopy = HEADER_LENGTH - headerpos;

					if (tocopy > available)
						tocopy = available;

					memcpy(header + headerpos, bytes, tocopy);
					headerpos += tocopy;
					bytes += tocopy;
					available -= tocopy;

					if (headerpos < HEADER_LENGTH)
						return Result::Consumed;

					length = decode_length(header);
					pos = 0;

					if (length > MAX_PACKET_LENGTH)
					{
						reset();
						return Result::InvalidLength;
					}
				}

				if (pos < length)
				{
					size_t towrite = length - pos;

					if (towrite > available)
						towrite = available;

					memcpy(buffer + pos, bytes, towrite);
					pos += towrite;
					bytes += towrite;
					available -= towrite;
				}

				// A frame with an empty body is complete once its header is read.
				if (pos < length)
					return Result::Consumed;

				size_t framelength = length;

				// Reset first so the callback can never observe or corrupt partial state.
				reset();

				if (!on_frame(buffer, framelength))
					return Result::Stopped;
			}

			return Result::Consumed;
		}

	private:
		int8_t header[HEADER_LENGTH];
		size_t headerpos;
		int8_t buffer[MAX_PACKET_LENGTH];
		size_t length;
		size_t pos;
	};
}
