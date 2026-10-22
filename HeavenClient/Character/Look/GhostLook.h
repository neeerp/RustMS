#pragma once

#include <cstdint>
#include <map>
#include <string>
#include <unordered_map>
#include <utility>
#include <vector>

#include "../../Graphics/Texture.h"

namespace ms
{
	// A ghost item's body (Character.nx's ghost* actions of the skin's body, report c039): only a body, with
	// the head drawn on its neck.
	class GhostLook
	{
	public:
		// The ghost of the skin `skin` for a ghost item's value `ghost`, loaded once.
		static const GhostLook &get(int32_t skin, int16_t ghost);

		GhostLook(int32_t skin, int16_t ghost);

		void draw(const std::string &action, uint8_t frame, const DrawArgument &args) const;

		// How many frames an action has, 0 for none.
		uint8_t count(const std::string &action) const;

		uint16_t delay(const std::string &action, uint8_t frame) const;

		// Whether the frame faces the front, with the face drawn, or shows the back of the head.
		bool faces(const std::string &action, uint8_t frame) const;

		// How far the head moves from where it is on the body standing, or facing a ladder for a frame
		// that shows its back.
		Point<int16_t> neck(const std::string &action, uint8_t frame) const;

	private:
		struct Frame
		{
			Texture texture;
			uint16_t delay;
			Point<int16_t> neck;
			bool face;
		};

		const Frame *find(const std::string &action, uint8_t frame) const;

		std::unordered_map<std::string, std::vector<Frame>> actions;

		static std::map<std::pair<int32_t, int16_t>, GhostLook> cache;
	};
}
