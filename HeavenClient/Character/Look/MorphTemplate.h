#pragma once

#include <cstdint>
#include <string>
#include <unordered_map>
#include <vector>

#include "../../Graphics/Texture.h"

namespace ms
{
	// A Morph.nx template, drawn in place of the body and equipment (FUN_00688900).
	class MorphTemplate
	{
	public:
		// The template `id`, loaded once.
		static const MorphTemplate &get(int32_t id);

		MorphTemplate(nl::node src);

		// Draw an action's frame.
		void draw(const std::string &action, uint8_t frame, const DrawArgument &args) const;

		// The template's action for a stance's name, or stand without it.
		std::string action(const std::string &stance) const;

		// How many frames an action has, 0 for none.
		uint8_t count(const std::string &action) const;

		uint16_t delay(const std::string &action, uint8_t frame) const;

		// An item's morph: neither a superman's (Transformation) nor hidden (Oak Barrel) (FUN_008c46de).
		bool is_item() const;

		// Oak Barrel's, which hides the player from mobs (FUN_004520d5).
		bool is_hide() const;

		// Transformation's, which keeps the player's own stats and attacks.
		bool is_superman() const;

		int32_t get_speed() const;

		int32_t get_jump() const;

		// The grip on the ground, for the walk's force and drag.
		double get_fs() const;

		// How fast it swims, as a percent of its speed.
		int32_t get_swim() const;

	private:
		struct Frame
		{
			Texture texture;
			uint16_t delay;
		};

		const Frame *find(const std::string &action, uint8_t frame) const;

		std::unordered_map<std::string, std::vector<Frame>> actions;
		int32_t speed;
		int32_t jump;
		double fs;
		int32_t swim;
		bool superman;
		bool hide;

		static std::unordered_map<int32_t, MorphTemplate> cache;
	};
}
