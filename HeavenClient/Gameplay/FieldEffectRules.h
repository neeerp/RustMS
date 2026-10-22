#pragma once

#include <cmath>
#include <cstdint>
#include <optional>
#include <string>

// The numbers behind v83's FIELD_EFFECT (0x8A) modes (summons, screen effects and BGM changes) and its back fades.
namespace ms::field_effect
{
	// A summon's sound by its distance from the player: full within 250 px, falling to 40 at 1000 px
	// (FUN_009894f3, which truncates).
	inline uint8_t summon_volume(double distance)
	{
		if (distance < 250)
			return 100;

		if (distance <= 1000)
			return static_cast<uint8_t>(120 - 0.08 * distance);

		return 40;
	}

	struct ScreenPosition
	{
		int16_t x;
		int16_t y;
	};

	// A screen effect plays at (400, 260) in an 800x600 view, 40 px above its centre.
	inline ScreenPosition screen_position(int16_t width, int16_t height)
	{
		return { static_cast<int16_t>(width / 2), static_cast<int16_t>(height / 2 - 40) };
	}

	// FIELD_EFFECT 6 names a BGM as "Bgm09/TimeAttack", which is Sound.nx Bgm09.img/TimeAttack.
	inline std::string bgm_path(const std::string &name)
	{
		size_t split = name.find('/');

		if (split == std::string::npos)
			return "";

		return name.substr(0, split) + ".img/" + name.substr(split + 1);
	}

	// A user effect's path names an NX file, then a node in it ("Effect/BasicEff.img/Event1/Best"). Item.nx's
	// reward paths leave out the image's ".img" ("Effect/BasicEff/Event1/Best"), so the `node` is also looked for
	// with it put back after the image's name: this path, or "" if `node` already has an ".img" or names no node.
	inline std::string with_image(const std::string &node)
	{
		if (node.find(".img") != std::string::npos)
			return "";

		return bgm_path(node);
	}

	// A changed BGM fades out over this many ms before the next one starts (PlayBGM 0x43f301).
	constexpr int32_t BGM_FADE = 600;

	// v83 lowers the BGM's volume (0 to 128) linearly to 0 and plays a volume v at
	// 625 - 1280000 / (15 v + 128) hundredths of a dB (Sound_DX8 0x5180c8a6). This is the gain left
	// after `elapsed` ms of a fade from full volume.
	inline float fade_gain(int32_t elapsed, int32_t duration)
	{
		if (elapsed <= 0)
			return 1.0f;

		if (elapsed >= duration)
			return 0.0f;

		double volume = 128.0 * (duration - elapsed) / duration;
		double hundredths = 625 - 1280000 / (15 * volume + 128);

		return static_cast<float>(std::pow(10.0, hundredths / 2000));
	}

	// The gain `elapsed` ms into a rise from silence that takes `duration` ms: a fade run backwards.
	inline float fade_in_gain(int32_t elapsed, int32_t duration)
	{
		return fade_gain(duration - elapsed, duration);
	}

	// PLAY_JUKEBOX (0x8F) plays only 510xxxx items' songs (cash type 0x14 in FUN_004863d5).
	inline bool is_jukebox(int32_t item_id)
	{
		return item_id / 10000 == 510;
	}

	// The map's BGM fades out over 1500 ms. 2500 ms after the packet the song starts, rising over 1000 ms, and it loops
	// until the player leaves the map (FUN_00641ecc, FUN_00641db2).
	constexpr int32_t JUKEBOX_FADE = 1500;
	constexpr int32_t JUKEBOX_DELAY = 2500;
	constexpr int32_t JUKEBOX_FADE_IN = 1000;

	// A jukebox's info/path names its song from the top of Sound.nx: "Sound/Jukebox.img/Congratulation".
	inline std::string jukebox_path(const std::string &path)
	{
		const std::string prefix = "Sound/";

		if (path.compare(0, prefix.size(), prefix) == 0)
			return path.substr(prefix.size());

		return path;
	}

	// A sound's length in ms from its 82-byte WZ header: the bytes after it over the byte rate at byte 60 (WAVEFORMATEX
	// nAvgBytesPerSec). Congratulation is 670302 bytes at 10000 a second, 67030 ms. 0 without a header or a rate.
	inline int32_t sound_length(const uint8_t *data, uint32_t length)
	{
		if (!data || length <= 82)
			return 0;

		uint32_t rate = data[60] | data[61] << 8 | data[62] << 16 | static_cast<uint32_t>(data[63]) << 24;

		if (rate == 0)
			return 0;

		return static_cast<int32_t>(static_cast<uint64_t>(length - 82) * 1000 / rate);
	}

	// The item's String Cash.img msg with the player's name for %s: "%s's song for you." (v83 sprintf, chat type 9).
	inline std::string jukebox_line(const std::string &msg, const std::string &name)
	{
		size_t at = msg.find("%s");

		if (at == std::string::npos)
			return msg;

		return msg.substr(0, at) + name + msg.substr(at + 2);
	}

	// SET_BACK_EFFECT (0x80) fades a back in to full (mode 0) or out to nothing (mode 1). Other modes change nothing
	// (FUN_006445c5).
	inline std::optional<float> back_target(uint8_t mode)
	{
		if (mode == 0)
			return 255.0f;

		if (mode == 1)
			return 0.0f;

		return std::nullopt;
	}

	// A back's alpha `elapsed` ms into a fade from `from` to `to` that takes `duration` ms. Gr2D moves it linearly.
	inline float back_alpha(float from, float to, int32_t elapsed, int32_t duration)
	{
		if (elapsed >= duration)
			return to;

		return from + (to - from) * elapsed / duration;
	}
}
