#include "TamingMob.h"

#include "../MountRules.h"

#include "../../Util/Misc.h"

#include <nlnx/nx.hpp>

namespace ms
{
	TamingMob::TamingMob(nl::node src)
	{
		for (nl::node actionnode : src)
		{
			std::vector<Frame> &frames = actions[actionnode.name()];

			for (uint8_t f = 0; nl::node framenode = actionnode[std::to_string(f)]; ++f)
			{
				Frame frame{ {}, static_cast<uint16_t>(framenode["delay"].get_integer(0)), {} };

				if (frame.delay == 0)
					frame.delay = 100;

				for (uint8_t p = 0; nl::node partnode = framenode[std::to_string(p)]; ++p)
				{
					if (p == 0)
						frame.navel = partnode["map"]["navel"];

					int8_t z = mount::z_index(partnode["z"].get_string());

					if (partnode.data_type() == nl::node::type::bitmap && z >= 0)
						frame.parts.push_back({ partnode, z });
				}

				frames.push_back(std::move(frame));
			}
		}
	}

	const TamingMob &TamingMob::get(int32_t id, int32_t mount)
	{
		auto key = std::make_pair(id, mount);
		auto iter = cache.find(key);

		if (iter == cache.end())
		{
			nl::node src = nl::nx::character["TamingMob"][string_format::extend_id(id, 8) + ".img"];

			if (mount > 0)
				src = src[std::to_string(mount)];

			iter = cache.emplace(key, src).first;
		}

		return iter->second;
	}

	bool TamingMob::saddle_fits(int32_t saddle, int32_t monster)
	{
		// The saddle's img lists the monsters it fits, beside its info.
		std::vector<int32_t> listed;

		for (const nl::node &child : nl::nx::character["TamingMob"][string_format::extend_id(saddle, 8) + ".img"])
			if (child.name() != "info")
				listed.push_back(std::atoi(child.name().c_str()));

		return mount::saddle_fits(saddle, monster, listed);
	}

	std::optional<TamingMob::Ability> TamingMob::ability(int32_t mount, int32_t saddle)
	{
		nl::node items = nl::nx::character["TamingMob"];
		int32_t id = items[string_format::extend_id(mount, 8) + ".img"]["info"]["tamingMob"];
		nl::node info = nl::nx::tamingmob[string_format::extend_id(id, 4) + ".img"]["info"];

		if (id <= 0 || !info)
			return std::nullopt;

		nl::node bonus = items[string_format::extend_id(saddle, 8) + ".img"]["info"];

		return Ability{
			mount::speed(info["speed"], bonus["incSpeed"]),
			mount::jump(info["jump"], bonus["incJump"]),
			info["fs"].get_real(1.0),
			static_cast<int32_t>(info["swim"].get_integer(100))
		};
	}

	void TamingMob::draw(const std::string &action, uint8_t frame, int8_t z, const DrawArgument &args) const
	{
		if (const Frame *f = find(action, frame))
			for (const Part &part : f->parts)
				if (part.z == z)
					part.texture.draw(args);
	}

	uint8_t TamingMob::count(const std::string &action) const
	{
		auto iter = actions.find(action);

		return iter == actions.end() ? 0 : static_cast<uint8_t>(iter->second.size());
	}

	uint16_t TamingMob::delay(const std::string &action, uint8_t frame) const
	{
		const Frame *f = find(action, frame);

		return f ? f->delay : 100;
	}

	Point<int16_t> TamingMob::navel(const std::string &action, uint8_t frame) const
	{
		const Frame *f = find(action, frame);

		return f ? f->navel : Point<int16_t>();
	}

	const TamingMob::Frame *TamingMob::find(const std::string &action, uint8_t frame) const
	{
		auto iter = actions.find(action);

		if (iter == actions.end() || iter->second.empty())
			return nullptr;

		return &iter->second[frame % iter->second.size()];
	}

	std::map<std::pair<int32_t, int32_t>, TamingMob> TamingMob::cache;
}
