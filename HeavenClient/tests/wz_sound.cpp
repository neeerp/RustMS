#include "../Audio/WzSound.h"

#include <cstdint>
#include <cstring>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

namespace
{
	void require(bool condition, const char* message)
	{
		if (!condition)
			throw std::runtime_error(message);
	}

	uint32_t u32(const std::vector<char>& bytes, size_t at)
	{
		uint32_t value = 0;

		for (int i = 0; i < 4; i++)
			value |= static_cast<uint32_t>(static_cast<uint8_t>(bytes[at + i])) << (8 * i);

		return value;
	}

	// A WZ sound header with this WAVEFORMATEX, then the samples.
	std::vector<char> stored(const std::vector<uint8_t>& format, const std::string& samples)
	{
		std::vector<char> data(51, '\0');
		data.push_back(static_cast<char>(format.size()));
		data.insert(data.end(), format.begin(), format.end());
		data.insert(data.end(), samples.begin(), samples.end());

		return data;
	}
}

int main()
{
	using namespace ms;

	try
	{
		// Mono 16 bit 44100 Hz PCM, as Mob.img/9300157/Die (18 bytes, cbSize 0).
		std::vector<uint8_t> pcm = { 1, 0, 1, 0, 0x44, 0xAC, 0, 0, 0x88, 0x58, 1, 0, 2, 0, 16, 0, 0, 0 };
		std::vector<char> data = stored(pcm, "abcdef");
		std::vector<char> wave = wzsound::wave_file(data.data(), data.size());

		require(wave.size() == 44 + 6, "a header and the samples");
		require(std::string(wave.begin(), wave.begin() + 4) == "RIFF", "RIFF");
		require(u32(wave, 4) == 36 + 6, "the RIFF size");
		require(std::string(wave.begin() + 8, wave.begin() + 16) == "WAVEfmt ", "WAVE and fmt");
		require(u32(wave, 16) == 16, "a 16 byte format");
		require(std::memcmp(wave.data() + 20, pcm.data(), 16) == 0, "the format as stored");
		require(std::string(wave.begin() + 36, wave.begin() + 40) == "data", "data");
		require(u32(wave, 40) == 6, "the data size");
		require(std::string(wave.begin() + 44, wave.end()) == "abcdef", "the samples");

		// MP3 (0x55), as UI.img/ScrollOpen: alure reads it as it is.
		std::vector<uint8_t> mp3(30, 0);
		mp3[0] = 0x55;
		data = stored(mp3, "frames");
		require(wzsound::wave_file(data.data(), data.size()).empty(), "MP3 isn't wrapped");

		// PCM whose format runs past the end, or is shorter than a PCMWAVEFORMAT.
		data = stored(pcm, "");
		data.resize(data.size() - 2);
		require(wzsound::wave_file(data.data(), data.size()).empty(), "a cut off format");
		std::vector<uint8_t> short_format(pcm.begin(), pcm.begin() + 14);
		data = stored(short_format, "abcdef");
		require(wzsound::wave_file(data.data(), data.size()).empty(), "a short format");

		require(wzsound::wave_file(nullptr, 0).empty(), "no sound");
		data.resize(40);
		require(wzsound::wave_file(data.data(), data.size()).empty(), "too short for a header");

		std::cout << "wz_sound: ok\n";
		return 0;
	}
	catch (const std::exception& e)
	{
		std::cerr << "wz_sound: " << e.what() << '\n';
		return 1;
	}
}
