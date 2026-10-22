#include "GhostLook.h"

#include "../MorphRules.h"

#include "../../Util/Misc.h"

#include <nlnx/nx.hpp>

namespace ms
{
	GhostLook::GhostLook(int32_t skin, int16_t ghost)
	{
		nl::node bodynode = nl::nx::character["000020" + string_format::extend_id(skin, 2) + ".img"];

		if (!bodynode["ghoststand"])
			bodynode = nl::nx::character["00002000.img"];

		Point<int16_t> standing = bodynode["stand1"]["0"]["body"]["map"]["neck"];
		Point<int16_t> climbing = bodynode["ladder"]["0"]["body"]["map"]["neck"];

		for (const char *action : morph::GHOST_ACTIONS)
		{
			std::vector<Frame> &frames = actions[action];
			nl::node variant = bodynode[action][morph::ghost_variant(ghost)];

			for (uint8_t f = 0; nl::node framenode = variant[std::to_string(f)]; ++f)
			{
				// The body is the frame's `body` canvas, or its other one (ghostsit/1/0's is named `0`).
				// ghostproneStab/1 also has an `arm`, put on the body's navel.
				nl::node canvas = framenode["body"];
				nl::node armnode = framenode["arm"];

				if (canvas.data_type() != nl::node::type::bitmap)
					for (nl::node part : framenode)
						if (part.data_type() == nl::node::type::bitmap && part.name() != "arm")
							canvas = part;

				Texture arm;

				if (armnode.data_type() == nl::node::type::bitmap)
				{
					arm = Texture(armnode);
					arm.shift(Point<int16_t>(canvas["map"]["navel"]) - Point<int16_t>(armnode["map"]["navel"]));
				}

				// A frame's delay, else its body's, as ghostsit/2/0 has.
				auto delay = static_cast<uint16_t>(framenode["delay"].get_integer(canvas["delay"].get_integer(0)));
				bool face = framenode["face"].get_bool();
				Point<int16_t> neck = canvas["map"]["neck"];

				frames.push_back({ canvas, arm, delay > 0 ? delay : static_cast<uint16_t>(100),
					neck - (face ? standing : climbing), face });
			}
		}
	}

	const GhostLook &GhostLook::get(int32_t skin, int16_t ghost)
	{
		auto key = std::make_pair(skin, ghost);
		auto iter = cache.find(key);

		if (iter == cache.end())
			iter = cache.emplace(key, GhostLook(skin, ghost)).first;

		return iter->second;
	}

	void GhostLook::draw(const std::string &action, uint8_t frame, const DrawArgument &args) const
	{
		if (const Frame *f = find(action, frame))
			f->texture.draw(args);
	}

	void GhostLook::draw_arm(const std::string &action, uint8_t frame, const DrawArgument &args) const
	{
		if (const Frame *f = find(action, frame))
			f->arm.draw(args);
	}

	uint8_t GhostLook::count(const std::string &action) const
	{
		auto iter = actions.find(action);

		return iter == actions.end() ? 0 : static_cast<uint8_t>(iter->second.size());
	}

	uint16_t GhostLook::delay(const std::string &action, uint8_t frame) const
	{
		const Frame *f = find(action, frame);

		return f ? f->delay : 100;
	}

	bool GhostLook::faces(const std::string &action, uint8_t frame) const
	{
		const Frame *f = find(action, frame);

		return !f || f->face;
	}

	Point<int16_t> GhostLook::neck(const std::string &action, uint8_t frame) const
	{
		const Frame *f = find(action, frame);

		return f ? f->neck : Point<int16_t>{};
	}

	const GhostLook::Frame *GhostLook::find(const std::string &action, uint8_t frame) const
	{
		auto iter = actions.find(action);

		if (iter == actions.end() || frame >= iter->second.size())
			return nullptr;

		return &iter->second[frame];
	}

	std::map<std::pair<int32_t, int16_t>, GhostLook> GhostLook::cache;
}
