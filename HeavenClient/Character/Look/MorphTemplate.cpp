#include "MorphTemplate.h"

#include "../MorphRules.h"

#include "../../Util/Misc.h"

#include <nlnx/nx.hpp>

namespace ms
{
	MorphTemplate::MorphTemplate(nl::node src)
	{
		nl::node info = src["info"];
		speed = static_cast<int32_t>(info["speed"].get_integer(100));
		jump = static_cast<int32_t>(info["jump"].get_integer(100));
		fs = info["fs"].get_real(1.0);
		swim = static_cast<int32_t>(info["swim"].get_integer(100));
		superman = info["superman"].get_bool();
		hide = info["hide"].get_bool();

		for (nl::node actionnode : src)
		{
			if (actionnode.name() == "info")
				continue;

			std::vector<Frame> &frames = actions[actionnode.name()];

			for (uint8_t f = 0; nl::node framenode = actionnode[std::to_string(f)]; ++f)
			{
				auto delay = static_cast<uint16_t>(framenode["delay"].get_integer(0));
				frames.push_back({ framenode, delay > 0 ? delay : static_cast<uint16_t>(100) });
			}
		}
	}

	const MorphTemplate &MorphTemplate::get(int32_t id)
	{
		auto iter = cache.find(id);

		if (iter == cache.end())
			iter = cache.emplace(id, nl::nx::morph[string_format::extend_id(id, 4) + ".img"]).first;

		return iter->second;
	}

	void MorphTemplate::draw(const std::string &action, uint8_t frame, const DrawArgument &args) const
	{
		if (const Frame *f = find(action, frame))
			f->texture.draw(args);
	}

	std::string MorphTemplate::action(const std::string &stance) const
	{
		std::string name = morph::action(stance);

		return count(name) > 0 ? name : "stand";
	}

	uint8_t MorphTemplate::count(const std::string &action) const
	{
		auto iter = actions.find(action);

		return iter == actions.end() ? 0 : static_cast<uint8_t>(iter->second.size());
	}

	uint16_t MorphTemplate::delay(const std::string &action, uint8_t frame) const
	{
		const Frame *f = find(action, frame);

		return f ? f->delay : 100;
	}

	bool MorphTemplate::is_item() const
	{
		return !actions.empty() && !superman && !hide;
	}

	int32_t MorphTemplate::get_speed() const
	{
		return speed;
	}

	int32_t MorphTemplate::get_jump() const
	{
		return jump;
	}

	double MorphTemplate::get_fs() const
	{
		return fs;
	}

	int32_t MorphTemplate::get_swim() const
	{
		return swim;
	}

	const MorphTemplate::Frame *MorphTemplate::find(const std::string &action, uint8_t frame) const
	{
		auto iter = actions.find(action);

		if (iter == actions.end() || iter->second.empty())
			return nullptr;

		return &iter->second[frame % iter->second.size()];
	}

	std::unordered_map<int32_t, MorphTemplate> MorphTemplate::cache;
}
