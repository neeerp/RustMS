#pragma once

#include <cmath>
#include <cstdint>
#include <string>

// The numbers behind v83's FIELD_EFFECT (0x8A) modes: summons, screen effects and BGM changes.
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
}
