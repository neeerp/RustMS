#include "WzSound.h"

#include <cstdint>
#include <cstring>

namespace ms
{
	namespace wzsound
	{
		namespace
		{
			constexpr size_t FORMAT_LENGTH = 51;
			constexpr size_t FORMAT = 52;
			// WAVE_FORMAT_PCM, and the size of the PCMWAVEFORMAT a WAV's
			// "fmt " chunk holds.
			constexpr uint16_t PCM = 1;
			constexpr uint32_t PCM_FORMAT_SIZE = 16;

			void put(std::vector<char>& out, const char* bytes, size_t count)
			{
				out.insert(out.end(), bytes, bytes + count);
			}

			void put(std::vector<char>& out, uint32_t value)
			{
				for (int i = 0; i < 4; i++)
					out.push_back(static_cast<char>((value >> (8 * i)) & 0xFF));
			}
		}

		std::vector<char> wave_file(const char* data, size_t length)
		{
			if (!data || length < FORMAT + PCM_FORMAT_SIZE)
				return {};

			auto bytes = reinterpret_cast<const uint8_t*>(data);
			size_t format_length = bytes[FORMAT_LENGTH];
			size_t start = FORMAT + format_length;
			uint16_t format = static_cast<uint16_t>(bytes[FORMAT] | (bytes[FORMAT + 1] << 8));

			if (format != PCM || format_length < PCM_FORMAT_SIZE || start > length)
				return {};

			uint32_t samples = static_cast<uint32_t>(length - start);
			std::vector<char> wave;
			wave.reserve(44 + samples);

			put(wave, "RIFF", 4);
			put(wave, 36 + samples);
			put(wave, "WAVE", 4);
			put(wave, "fmt ", 4);
			put(wave, PCM_FORMAT_SIZE);
			put(wave, data + FORMAT, PCM_FORMAT_SIZE);
			put(wave, "data", 4);
			put(wave, samples);
			put(wave, data + start, samples);

			return wave;
		}
	}
}
