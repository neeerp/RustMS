#pragma once

#include <cstdint>
#include <map>
#include <optional>
#include <string>
#include <unordered_map>
#include <utility>
#include <vector>

#include "../../Graphics/Texture.h"

namespace ms
{
	// The frames of a mount (190xxxx or 193xxxx), or of a saddle on one mount, from Character/TamingMob
	// (FUN_00413a43, report 201).
	class TamingMob
	{
	public:
		// How a mount moves, from its TamingMob.nx template and the saddle (FUN_0094d3d9).
		struct Ability
		{
			int32_t speed;
			int32_t jump;
			// The mount's grip on the ground, for the walk's force and drag.
			double fs;
			// How fast it swims, as a percent of its speed.
			int32_t swim;
		};

		// The mount `id`, or with a `mount`, the saddle `id` on that mount. Each is loaded once.
		static const TamingMob &get(int32_t id, int32_t mount = 0);

		// How riding `mount` with `saddle` moves, or nothing if the mount has no template.
		static std::optional<Ability> ability(int32_t mount, int32_t saddle);

		TamingMob(nl::node src);

		// Draw an action's frame's parts at place `z` of mount::ZMAP.
		void draw(const std::string &action, uint8_t frame, int8_t z, const DrawArgument &args) const;

		// How many frames an action has, 0 for none.
		uint8_t count(const std::string &action) const;

		uint16_t delay(const std::string &action, uint8_t frame) const;

		// Where the rider's navel goes, from the feet.
		Point<int16_t> navel(const std::string &action, uint8_t frame) const;

	private:
		struct Part
		{
			Texture texture;
			int8_t z;
		};

		struct Frame
		{
			std::vector<Part> parts;
			uint16_t delay;
			Point<int16_t> navel;
		};

		const Frame *find(const std::string &action, uint8_t frame) const;

		std::unordered_map<std::string, std::vector<Frame>> actions;

		static std::map<std::pair<int32_t, int32_t>, TamingMob> cache;
	};
}
