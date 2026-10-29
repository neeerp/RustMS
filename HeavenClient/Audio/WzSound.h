#pragma once

#include <cstddef>
#include <vector>

namespace ms
{
	// A sound as the WZ files store it: a header whose byte 51 is the length
	// of the WAVEFORMATEX at byte 52, then the samples. Most are MP3, which
	// alure reads from byte 82 on; some (a mob's cries, for one) are PCM,
	// which it can't read without a RIFF header.
	namespace wzsound
	{
		// The PCM sound as a WAV file, or nothing for any other sound.
		std::vector<char> wave_file(const char* data, size_t length);
	}
}
