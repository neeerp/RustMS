#include "QuestCatalog.h"

#include <algorithm>
#include <fstream>
#include <iostream>

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		bool is_number(const std::string& name)
		{
			return !name.empty() && std::all_of(name.begin(), name.end(), [](char c) { return c >= '0' && c <= '9'; });
		}

		// Numbered children in order.
		std::vector<std::string> pages(nl::node node)
		{
			std::vector<std::pair<int64_t, std::string>> numbered;

			for (auto child : node)
				if (is_number(child.name()) && child.data_type() == nl::node::type::string)
					numbered.emplace_back(std::stoll(child.name()), child.get_string());

			std::sort(numbered.begin(), numbered.end());
			std::vector<std::string> result;

			for (auto& page : numbered)
				result.push_back(std::move(page.second));

			return result;
		}

		QuestInfo parse(nl::node root, int16_t id)
		{
			std::string name = std::to_string(id);
			nl::node start = root["Check.img"][name]["0"];
			nl::node act = root["Act.img"][name]["1"];

			QuestInfo info;
			info.id = id;
			info.name = root["QuestInfo.img"][name]["name"].get_string();
			info.start_npc = static_cast<int32_t>(start["npc"].get_integer());
			info.end_npc = static_cast<int32_t>(root["Check.img"][name]["1"]["npc"].get_integer());
			info.min_level = static_cast<int16_t>(start["lvmin"].get_integer(0));
			info.max_level = static_cast<int16_t>(start["lvmax"].get_integer(32767));
			info.exp = static_cast<int32_t>(act["exp"].get_integer());
			info.next_quest = static_cast<int16_t>(act["nextQuest"].get_integer());

			for (auto job : start["job"])
				info.jobs.push_back(static_cast<int16_t>(job.get_integer()));

			// A prerequisite without a state must not have been started.
			for (auto prerequisite : start["quest"])
				info.prerequisites.emplace_back(static_cast<int16_t>(prerequisite["id"].get_integer()),
					static_cast<int16_t>(prerequisite["state"].get_integer()));

			return info;
		}
	}

	// The server already refused every quest it can't run, so the list needs
	// no checks here.
	QuestCatalog::QuestCatalog(nl::node quest_root, std::istream& ids) : root(quest_root)
	{
		for (std::string line; std::getline(ids, line);)
			if (!line.empty() && line[0] != '#')
				supported.push_back(parse(root, static_cast<int16_t>(std::stoi(line))));

		std::sort(supported.begin(), supported.end(), [](const QuestInfo& a, const QuestInfo& b) { return a.id < b.id; });
	}

	const QuestCatalog& QuestCatalog::get()
	{
		static const QuestCatalog catalog = []()
		{
			std::ifstream ids("supported_quests.txt");

			if (!ids)
				std::cout << "supported_quests.txt not found; no quests will be offered" << std::endl;

			return QuestCatalog(nl::nx::quest, ids);
		}();

		return catalog;
	}

	const QuestInfo* QuestCatalog::find(int16_t id) const
	{
		auto iter = std::lower_bound(supported.begin(), supported.end(), id,
			[](const QuestInfo& quest, int16_t target) { return quest.id < target; });

		return iter == supported.end() || iter->id != id ? nullptr : &*iter;
	}

	const std::vector<QuestInfo>& QuestCatalog::quests() const
	{
		return supported;
	}

	QuestScript QuestCatalog::script(int16_t id, bool completion) const
	{
		nl::node say = root["Say.img"][std::to_string(id)][completion ? "1" : "0"];

		QuestScript script;
		script.pages = pages(say);
		script.yes = pages(say["yes"]);
		script.no = pages(say["no"]);

		for (auto stop : say["stop"])
		{
			if (stop.name() == "npc")
			{
				script.reminder = pages(stop);
				continue;
			}

			if (!is_number(stop.name()))
				continue;

			auto page = static_cast<size_t>(std::stoul(stop.name()));

			// The answer is one-based; #L ids are zero-based.
			if (stop["answer"].data_type() == nl::node::type::integer)
				script.answers[page] = static_cast<int32_t>(stop["answer"].get_integer()) - 1;

			for (auto reason : stop)
				if (is_number(reason.name()) && reason.data_type() == nl::node::type::string)
					script.wrong[page][std::stoi(reason.name())] = reason.get_string();
		}

		return script;
	}
}
