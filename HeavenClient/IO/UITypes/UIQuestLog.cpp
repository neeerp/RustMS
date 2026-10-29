//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published by	//
//	the Free Software Foundation, either version 3 of the License, or			//
//	(at your option) any later version.											//
//																				//
//	This program is distributed in the hope that it will be useful,				//
//	but WITHOUT ANY WARRANTY; without even the implied warranty of				//
//	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the				//
//	GNU Affero General Public License for more details.							//
//																				//
//	You should have received a copy of the GNU Affero General Public License	//
//	along with this program.  If not, see <https://www.gnu.org/licenses/>.		//
//////////////////////////////////////////////////////////////////////////////////
#include "UIQuestLog.h"
#include "UINotice.h"
#include "UIQuestHelper.h"
#include "UIWorldMap.h"

#include "../UI.h"

#include "../Components/AreaButton.h"
#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Character/QuestRequirements.h"
#include "../../Character/QuestCatalog.h"
#include "../../Character/QuestMarker.h"
#include "../../Character/QuestRecordEx.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/QuestPackets.h"
#include "../../Graphics/GraphicsGL.h"
#include "../../Util/Misc.h"
#include "../../Util/NpcTextMarkup.h"
#include "../../Util/NxFiles.h"
#include "../../Util/PathImage.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <cctype>
#include <ctime>
#include <limits>
#include <map>

namespace ms
{
	namespace
	{
		// The window, measured from period footage. The list is on the
		// left; the details' coordinates are from their own origin.
		constexpr Point<int16_t> CLOSE_POS = Point<int16_t>(227, 6);
		constexpr Point<int16_t> TAB_STRIP = Point<int16_t>(5, 23);
		constexpr int16_t TAB_FILL = 56;
		constexpr int16_t LIST_WIDTH = 245;
		constexpr int16_t DETAIL_X = 244;
		constexpr int16_t DETAIL_WIDTH = 305;
		constexpr int16_t HEIGHT = 396;
		// Rows: an area's bar, or a quest with its icon.
		constexpr int16_t ROW_TOP = 50;
		constexpr int16_t ROW_PITCH = 22;
		constexpr int16_t ROWS = 14;
		constexpr int16_t ROW_X = 10;
		constexpr int16_t ROW_RIGHT = 219;
		constexpr int16_t BAR_HEIGHT = 18;
		constexpr Point<int16_t> FOLD_BOX = Point<int16_t>(13, 2);
		constexpr Point<int16_t> ROW_ICON = Point<int16_t>(11, 2);
		// The in-progress arrow is narrower and stands a little lower.
		constexpr Point<int16_t> ARROW_ICON = Point<int16_t>(14, 3);
		constexpr int16_t LABEL_X = 30;
		constexpr int16_t LABEL_Y = -4;
		// Unselected labels run to the scroll bar; the selected one stops
		// short of its bar's end with "..".
		constexpr int16_t LABEL_WIDTH = 195;
		constexpr int16_t CHOSEN_WIDTH = 186;
		constexpr int16_t LIST_SLIDER_X = 225;
		constexpr Range<int16_t> LIST_SLIDER = Range<int16_t>(50, 346);
		// The details.
		constexpr Point<int16_t> HIDE_POS = Point<int16_t>(285, 6);
		constexpr Point<int16_t> TITLE_POS = Point<int16_t>(34, 34);
		constexpr int16_t TITLE_WIDTH = 152;
		constexpr Point<int16_t> JOBS_POS = Point<int16_t>(22, 57);
		constexpr Point<int16_t> LEVELS_POS = Point<int16_t>(22, 73);
		constexpr int16_t HEADER_WIDTH = 164;
		// The portrait's feet.
		constexpr Point<int16_t> PORTRAIT_POS = Point<int16_t>(236, 108);
		constexpr Point<int16_t> TEXT_POS = Point<int16_t>(18, 121);
		constexpr int16_t TEXT_WIDTH = 260;
		constexpr Range<int16_t> TEXT_VIEW = Range<int16_t>(123, 364);
		constexpr int16_t TEXT_STEP = 18;
		constexpr int16_t DETAIL_SLIDER_X = 283;
		constexpr Range<int16_t> DETAIL_SLIDER = Range<int16_t>(123, 352);
		// Over a summary (backgrnd4) the description takes 120 px, and the
		// summary 108 px below it, with a scroll bar of its own (0x87c388,
		// 0x87bae7).
		constexpr Range<int16_t> SHORT_TEXT_VIEW = Range<int16_t>(123, 241);
		constexpr Range<int16_t> SHORT_DETAIL_SLIDER = Range<int16_t>(123, 229);
		constexpr Point<int16_t> SUMMARY_POS = Point<int16_t>(18, 251);
		constexpr Range<int16_t> SUMMARY_VIEW = Range<int16_t>(249, 357);
		constexpr Range<int16_t> SUMMARY_SLIDER = Range<int16_t>(249, 345);
		constexpr Point<int16_t> HELPER_POS = Point<int16_t>(150, 372);
		constexpr Point<int16_t> FORFEIT_POS = Point<int16_t>(242, 372);
		constexpr Point<int16_t> FIND_NPC_POS = Point<int16_t>(217, 372);
		// Item icons take a 32x32 box in the description.
		constexpr int16_t ICON_SIZE = 32;

		// The list's area headings by QuestInfo.img/<id>/area. Most are read
		// from period footage; the rest name the region their quests are in.
		std::string area_name(int16_t area)
		{
			static const std::map<int16_t, std::string> names = {
				{ 6, "Hero With The Lost Memory" },
				{ 10, "Job" },
				{ 15, "Knights of Cygnus" },
				{ 20, "Maple Island" },
				{ 30, "Victoria Island" },
				{ 33, "Elnath Mt. + Aquaroad" },
				{ 37, "Ludus Lake" },
				{ 41, "Minar Forest" },
				{ 44, "Mu Lung + Nihal Desert" },
				{ 45, "Masteria" },
				{ 46, "Temple of Time" },
				{ 47, "Party Quest" },
				{ 48, "World Tour" },
				{ 49, "Malaysia" },
				{ 50, "Event" }
			};

			auto name = names.find(area);

			return name == names.end() ? "Etc." : name->second;
		}

		// `text` cut to `width`, with `tail` after it if cut.
		std::string fit(Text::Font font, const std::string& text, int16_t width, const std::string& tail)
		{
			Text measure(font, Text::Alignment::LEFT, Color::Name::BLACK, text, 0, false);

			if (measure.width() <= width)
				return text;

			std::string cut = text;

			while (!cut.empty())
			{
				cut.pop_back();
				measure.change_text(cut + tail);

				if (measure.width() <= width)
					break;
			}

			return cut + tail;
		}

		// A #j token (#jtry#, #jrank#...) from the quest's record beside the
		// quest log, or a title's count against its need (the party quests
		// at rank S against partyQuest_S, the quests completed against
		// questComplete), level or fame.
		std::string record_token(const std::string& key, int16_t quest_id)
		{
			Player& player = Stage::get().get_player();
			const auto& quests = player.get_quests();
			nl::node end = nl::nx::quest["Check.img"][std::to_string(quest_id)]["1"];
			QuestRecordEx::Counts counts;

			if (key == "scnt" || key == "perPqS" || key == "gaugePqS")
				counts.rank_s = { quests.rank_count("S"), static_cast<int32_t>(end["partyQuest_S"].get_integer()) };

			if (key == "cmpcnt" || key == "perQCnt" || key == "gaugeQCnt")
				counts.completed = { quests.completed_count(), static_cast<int32_t>(end["questComplete"].get_integer()) };

			counts.level = player.get_stats().get_stat(MapleStat::Id::LEVEL);
			// Fame is signed, kept as its 16 bits.
			counts.fame = static_cast<int16_t>(player.get_stats().get_stat(MapleStat::Id::FAME));
			counts.time_limit = static_cast<int32_t>(
				nl::nx::quest["QuestInfo.img"][std::to_string(quest_id)]["timeLimit2"].get_integer());

			return QuestRecordEx::token(key, [&](const std::string& k) { return quests.record_ex(quest_id, k); }, counts);
		}

		// A #Q token, the time left on a timed title (its started record
		// names the end), or a #D token, Diligent Explorer's play today.
		std::string time_token(char tag, const std::string& key, int16_t quest_id)
		{
			const auto& quests = Stage::get().get_player().get_quests();
			nl::node info = nl::nx::quest["QuestInfo.img"][std::to_string(quest_id)];

			if (tag == 'Q')
			{
				auto started = quests.started_quests().find(quest_id);
				std::string progress = started == quests.started_quests().end() ? std::string() : started->second;

				return QuestRecordEx::time_left(key, QuestRecordEx::expiry(progress), static_cast<int64_t>(std::time(nullptr)));
			}

			return QuestRecordEx::play_time(key, [&](const std::string& k) { return quests.record_ex(quest_id, k); },
				static_cast<int32_t>(info["dailyPlayTime"].get_integer()), static_cast<int32_t>(info["timeLimit2"].get_integer()));
		}

		// Fill in the quest-only placeholders, which NpcTextMarkup leaves:
		// #c how many of an item the player holds, #a a kill count
		// (#a<index># for this quest's mobs, or #a<quest><index>#), #R
		// a quest's record as it stands (#R27010# an explorer title's count of
		// places), #j a value from the quest's record beside the quest
		// log (`record_token`), and #Q and #D a timed title's time left and
		// play today (`time_token`). Line breaks become the renderer's escapes.
		std::string quest_counts(const std::string& text, int16_t quest_id)
		{
			auto& player = Stage::get().get_player();
			std::string out;

			for (size_t i = 0; i < text.size(); i++)
			{
				char c = text[i];

				if (c == '\r' || c == '\n')
				{
					if (c == '\r' && i + 1 < text.size() && text[i + 1] == '\n')
						i++;

					out += "\\n";
					continue;
				}

				size_t close = c == '#' && i + 2 < text.size() ? text.find('#', i + 2) : std::string::npos;
				char tag = i + 1 < text.size() ? text[i + 1] : 0;

				// The tags NpcTextMarkup expands pass whole, so their closing
				// '#' doesn't start a tag of its own (#t4000000#jewel#).
				std::string arg = close == std::string::npos ? std::string() : text.substr(i + 2, close - i - 2);
				std::string number = !arg.empty() && arg.back() == ':' ? arg.substr(0, arg.size() - 1) : arg;

				if (close != std::string::npos && tag != 0
					&& ((std::string("ptzomvis").find(tag) != std::string::npos && NpcTextMarkup::is_digits(number))
						|| ((tag == 'f' || tag == 'F') && arg.find('/') != std::string::npos)
						|| (tag == 'W' && NpcTextMarkup::is_key(arg))))
				{
					out.append(text, i, close - i + 1);
					i = close;
					continue;
				}

				if (close != std::string::npos && (tag == 'j' || tag == 'Q' || tag == 'D') && close - i - 2 <= 10)
				{
					std::string key = text.substr(i + 2, close - i - 2);

					if (!key.empty() && std::all_of(key.begin(), key.end(), [](char k) { return std::isalnum(static_cast<unsigned char>(k)); }))
					{
						out += tag == 'j' ? record_token(key, quest_id) : time_token(tag, key, quest_id);
						i = close;
						continue;
					}
				}

				if (close == std::string::npos || (tag != 'c' && tag != 'a' && tag != 'R')
					|| !NpcTextMarkup::is_digits(text.substr(i + 2, close - i - 2)) || close - i - 2 > (tag == 'R' ? 5 : 9))
				{
					out += c;
					continue;
				}

				int32_t id = std::stoi(text.substr(i + 2, close - i - 2));

				if (tag == 'c')
				{
					out += std::to_string(player.get_inventory().get_total_item_count(id));
				}
				else if (tag == 'R')
				{
					const auto& started = player.get_quests().started_quests();
					auto record = started.find(static_cast<int16_t>(id));
					out += record == started.end() || record->second.empty() ? "0" : record->second;
				}
				else
				{
					int16_t quest = id < 10 ? quest_id : static_cast<int16_t>(id / 10);
					const auto& started = player.get_quests().started_quests();
					auto progress = started.find(quest);
					auto kills = decode_kills(progress == started.end() ? std::string() : progress->second);
					size_t index = static_cast<size_t>(id % 10);
					out += std::to_string(index >= 1 && index <= kills.size() ? kills[index - 1] : 0);
				}

				i = close;
			}

			return out;
		}
	}

	std::string quest_text(const std::string& text, int16_t quest_id)
	{
		return quest_counts(text, quest_id);
	}

	void QuestTextImages::set(const std::vector<NpcTextMarkup::InlineImage>& tags, Text& description)
	{
		images.clear();
		std::vector<Point<int16_t>> boxes;

		if (!gauge_slice.is_valid())
			gauge_slice = path_image(QuestRecordEx::GAUGE_SLICE);

		for (const auto& tag : tags)
		{
			Image image;

			if ((tag.tag == 'i' || tag.tag == 'v') && tag.arg.size() <= 9 && ItemData::get(std::stoi(tag.arg)).is_valid())
			{
				image.texture = ItemData::get(std::stoi(tag.arg)).get_icon(false);
				image.size = Point<int16_t>(ICON_SIZE, ICON_SIZE);
				image.anchor = Point<int16_t>(0, ICON_SIZE);
			}
			else if (tag.tag == 'f' || tag.tag == 'F')
			{
				image.slices = QuestRecordEx::gauge_slices(tag.arg);
				image.texture = path_image(image.slices < 0 ? tag.arg : tag.arg.substr(0, tag.arg.rfind('/')));
				image.size = image.texture.get_dimensions();
				image.anchor = image.texture.get_origin();
			}

			// Anything else is left out.
			boxes.push_back(image.texture.is_valid() ? image.size : Point<int16_t>());
			images.push_back(image);
		}

		description.set_boxes(boxes);
	}

	void QuestTextImages::draw(const Text& description, Point<int16_t> text, Range<int16_t> clip) const
	{
		for (size_t i = 0; i < images.size(); i++)
		{
			const Image& image = images[i];
			Point<int16_t> box = text + description.box_position(i);

			if (!image.texture.is_valid() || box.y() < clip.first() || box.y() + image.size.y() > clip.second())
				continue;

			image.texture.draw(box + image.anchor);

			// A gauge's slices go one pixel apart from (2, 3) in its frame.
			for (int32_t slice = 0; slice < image.slices; slice++)
				gauge_slice.draw(box + Point<int16_t>(static_cast<int16_t>(slice + 2), 3) + gauge_slice.get_origin());
		}
	}

	UIQuestLog::UIQuestLog(const QuestLog& ql) : UIDragElement<PosQUEST>(), questlog(ql)
	{
		nl::node Quest = NxFiles::ui_classic["UIWindow.img"]["Quest"];
		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		list_background = Quest["backgrnd"];
		detail_background = Quest["backgrnd2"];
		summary_background = Quest["backgrnd4"];

		for (size_t i = 0; i < 3; i++)
			notices[i] = Quest["notice" + std::to_string(i)];

		fold_boxes[0] = Basic["BtMin"]["normal"]["0"];
		fold_boxes[1] = Basic["BtMax"]["normal"]["0"];
		available_icon = Quest["icon0"];
		progress_animation = Quest["icon2"];
		ready_animation = Quest["icon3"];
		complete_icon = Quest["icon4"];

		tabs = ClassicTabs(ClassicTabs::TAB2, Quest["Tab"]["enabled"], Quest["Tab"]["disabled"], TAB_FILL);

		for (uint16_t i = 0; i < 3; i++)
		{
			buttons[TAB0 + i] = std::make_unique<AreaButton>(TAB_STRIP + Point<int16_t>(tabs.pitch() * i, 0),
				Point<int16_t>(tabs.pitch(), tabs.height()));
			buttons[TAB0 + i]->set_tab();
		}

		Point<int16_t> detail = Point<int16_t>(DETAIL_X, 0);

		buttons[CLOSE] = std::make_unique<MapleButton>(Basic["BtClose"], CLOSE_POS);
		buttons[HIDE] = std::make_unique<MapleButton>(Basic["BtHide"], detail + HIDE_POS);
		buttons[FIND_NPC] = std::make_unique<MapleButton>(Quest["BtMarkNpc"], detail + FIND_NPC_POS);
		buttons[HELPER] = std::make_unique<MapleButton>(Quest["BtAlert"], detail + HELPER_POS);
		buttons[FORFEIT] = std::make_unique<MapleButton>(Quest["BtGiveup"], detail + FORFEIT_POS);
		set_button_help(HELPER, "Game/Button/QuestAlarm");

		list_slider = Slider(Slider::Type::CLASSIC, LIST_SLIDER, LIST_SLIDER_X, ROWS, 0,
			[&](bool up) { scroll_list(list_offset + (up ? -1 : 1)); });
		detail_slider = Slider(Slider::Type::CLASSIC, DETAIL_SLIDER, DETAIL_X + DETAIL_SLIDER_X, 1, 1,
			[&](bool up) { scroll_details(detail_offset + (up ? -1 : 1)); });
		summary_slider = Slider(Slider::Type::CLASSIC, SUMMARY_SLIDER, DETAIL_X + DETAIL_SLIDER_X, 1, 1,
			[&](bool up) { scroll_summary(summary_offset + (up ? -1 : 1)); });

		seen_revision = std::numeric_limits<uint64_t>::max();
		change_tab(TAB0);
		show_details(false);
		update();
	}

	void UIQuestLog::update_screen(int16_t width, int16_t height)
	{
		dimension = Point<int16_t>(details_open ? DETAIL_X + DETAIL_WIDTH : LIST_WIDTH, HEIGHT);
		dragarea = Point<int16_t>(dimension.x(), 20);

		if (position.x() < 0 || position.x() + dimension.x() > width)
			position.set_x(std::max<int16_t>(0, (width - dimension.x()) / 2));

		if (position.y() < 0 || position.y() + dimension.y() > height)
			position.set_y(std::max<int16_t>(0, (height - dimension.y()) / 2));
	}

	void UIQuestLog::update()
	{
		UIElement::update();

		progress_animation.update();
		ready_animation.update();

		if (selected >= 0)
			portrait.update();

		auto& stats = Stage::get().get_player().get_stats();
		int16_t job = stats.get_stat(MapleStat::JOB);
		int16_t level = stats.get_stat(MapleStat::LEVEL);

		if (seen_revision != questlog.revision() || seen_job != job || seen_level != level)
		{
			seen_revision = questlog.revision();
			seen_job = job;
			seen_level = level;
			refresh_list();
			return;
		}

		// Items picked up or used change what a quest still needs without
		// touching the quest log.
		if (tab == TAB1)
		{
			for (auto& row : rows)
				if (row.quest >= 0)
					row.icon = progress_icon(row.quest);

			if (selected >= 0 && objectives(selected) != shown_goals)
				write_details();
		}
	}

	std::vector<QuestObjective> UIQuestLog::objectives(int16_t id) const
	{
		const QuestInfo* quest = QuestCatalog::get().find(id);
		const auto& started = questlog.started_quests();
		auto progress = started.find(id);

		if (!quest || progress == started.end())
			return {};

		const Inventory& inventory = Stage::get().get_player().get_inventory();

		return quest_objectives(quest->end_mobs, quest->end_items, progress->second,
			[&](int32_t item_id) { return static_cast<int32_t>(inventory.get_total_item_count(item_id)); });
	}

	UIQuestLog::Icon UIQuestLog::progress_icon(int16_t id) const
	{
		if (!QuestCatalog::get().find(id))
			return Icon::IN_PROGRESS;

		auto goals = objectives(id);

		return std::all_of(goals.begin(), goals.end(), [](const QuestObjective& goal) { return goal.met(); })
			&& party_rank_met(*QuestCatalog::get().find(id), questlog.rank_count("S"))
			&& completed_met(*QuestCatalog::get().find(id), questlog.completed_count())
			? Icon::READY : Icon::IN_PROGRESS;
	}

	void UIQuestLog::refresh_list()
	{
		std::vector<int16_t> ids;
		auto& player = Stage::get().get_player();
		auto& stats = player.get_stats();

		if (tab == TAB0)
			ids = questlog.available_quests(stats.get_stat(MapleStat::JOB), stats.get_stat(MapleStat::LEVEL),
				[&](const QuestInfo& quest) { return player.quest_start_met(quest); });
		else if (tab == TAB1)
			for (const auto& quest : questlog.started_quests())
				ids.push_back(quest.first);
		else
			for (const auto& quest : questlog.completed_quests())
				ids.push_back(quest.first);

		// Quests by area, the areas in order.
		std::map<int16_t, std::vector<std::pair<int16_t, std::string>>> areas;

		for (int16_t id : ids)
		{
			nl::node info = nl::nx::quest["QuestInfo.img"][std::to_string(id)];
			std::string name = info["name"];

			// Titles (area 51) are listed in the title window instead.
			if (!name.empty() && info["area"].get_integer() != QuestCatalog::TITLE_AREA)
				areas[static_cast<int16_t>(info["area"].get_integer())].emplace_back(id, name);
		}

		rows.clear();

		for (const auto& [area, quests] : areas)
		{
			std::string heading = area_name(area) + " (" + std::to_string(quests.size()) + ")";
			rows.push_back({ area, -1, Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, heading, 0, false),
				Text(), Icon::AVAILABLE });

			if (folded[tab].count(area))
				continue;

			for (const auto& [id, name] : quests)
			{
				Icon icon = tab == TAB0 ? Icon::AVAILABLE : tab == TAB1 ? progress_icon(id) : Icon::COMPLETE;
				Text label(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
					fit(Text::Font::A12M, name, LABEL_WIDTH, ""), 0, false);
				Text chosen(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE,
					fit(Text::Font::A12M, name, CHOSEN_WIDTH, ".."), 0, false);

				rows.push_back({ area, id, std::move(label), std::move(chosen), icon });
			}
		}

		int16_t count = static_cast<int16_t>(rows.size());
		list_offset = std::clamp<int16_t>(list_offset, 0, std::max<int16_t>(0, count - ROWS));
		list_slider.setrows(list_offset, ROWS, count);

		bool listed = std::any_of(rows.begin(), rows.end(), [&](const Row& row) { return row.quest == selected; });
		select_quest(listed ? selected : -1);
	}

	void UIQuestLog::select_quest(int16_t id)
	{
		selected = id;
		detail_offset = 0;
		summary_offset = 0;

		if (selected < 0)
		{
			start_npc = 0;
			show_details(false);
			return;
		}

		std::string name = std::to_string(id);
		nl::node info = nl::nx::quest["QuestInfo.img"][name];
		nl::node start = nl::nx::quest["Check.img"][name]["0"];

		// The portrait and FIND NPC show the one who gives it on every tab,
		// or Maple Administrator when the data names none (v83 FUN_0087c5e1).
		start_npc = start["npc"];

		if (start_npc == 0)
			start_npc = 9010000;

		// Its own portrait image when it has one, else it standing.
		nl::node face = nl::nx::npc[string_format::extend_id(start_npc, 7) + ".img"]["info"]["default"];
		portrait = face.data_type() == nl::node::type::bitmap ? face : NxHelper::Npc::get_npc_node(start_npc)["stand"];

		title = Text(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::WHITE,
			fit(Text::Font::A12B, info["name"], TITLE_WIDTH, ".."), 0, false);

		// The job ids in the data's order: by number, where the NX file sorts their names as text.
		std::vector<std::pair<int32_t, int32_t>> allowed;

		for (auto job : start["job"])
			allowed.emplace_back(string_conversion::or_zero<int32_t>(job.name()), static_cast<int32_t>(job.get_integer()));

		std::sort(allowed.begin(), allowed.end());

		std::vector<int32_t> ids;

		for (const auto& [order, job] : allowed)
			ids.push_back(job);

		jobs = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE,
			fit(Text::Font::A12M, QuestRequirements::jobs(ids), HEADER_WIDTH, ".."), 0, false);

		// An empty start check still counts (2049, 3240); only a missing one is "No limit".
		levels = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE,
			QuestRequirements::levels(static_cast<bool>(start), static_cast<int32_t>(start["lvmin"].get_integer()),
				static_cast<int32_t>(start["lvmax"].get_integer())), 0, false);

		write_details();
		show_details(true);
	}

	void UIQuestLog::write_details()
	{
		shown_goals = tab == TAB1 ? objectives(selected) : std::vector<QuestObjective>();

		// What the quest says in its current state: offered, started or done.
		std::string text = nl::nx::quest["QuestInfo.img"][std::to_string(selected)][std::to_string(tab)];

		NpcTextMarkup::Lookups lookups {
			[]() { return Stage::get().get_player().get_name(); },
			[](int32_t id) { return std::string(nl::nx::string["Npc.img"][std::to_string(id)]["name"]); },
			[](int32_t id) { return ItemData::get(id).get_name(); },
			[](int32_t id) { return std::string(nl::nx::string["Mob.img"][std::to_string(id)]["name"]); },
			[](int32_t id) { return NxHelper::Map::get_map_info_by_id(id).name; }
		};

		std::vector<NpcTextMarkup::InlineImage> tags;
		links.clear();
		text = NpcTextMarkup::expand(quest_counts(text, selected), lookups, &tags, &links);

		description = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, text, TEXT_WIDTH);
		images.set(tags, description);

		// In progress, the summary's paragraphs, each on lines of its own:
		// the "summary" heading, the summary, what the quest asks, then a
		// gap, the "reward" heading and what it gives. The client builds
		// none for the other tabs (0x881261).
		std::vector<std::string> paragraphs;

		if (tab == TAB1)
		{
			nl::node info = nl::nx::quest["QuestInfo.img"][std::to_string(selected)];

			for (const char* key : { "summary", "demandSummary", "rewardSummary" })
			{
				std::string paragraph = info[key];

				if (paragraph.empty())
					continue;

				if (std::string(key) == "rewardSummary")
				{
					paragraphs.push_back(" ");
					paragraphs.push_back("#Wreward#");
				}

				paragraphs.push_back(paragraph);
			}

			if (!paragraphs.empty())
				paragraphs.insert(paragraphs.begin(), "#Wsummary#");
		}

		std::string joined;

		for (const std::string& paragraph : paragraphs)
			joined += (joined.empty() ? "" : "\\n") + quest_counts(paragraph, selected);

		std::vector<NpcTextMarkup::InlineImage> summary_tags;
		has_summary = !paragraphs.empty();
		summary = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
			NpcTextMarkup::expand(joined, lookups, &summary_tags), TEXT_WIDTH);
		summary_images.set(summary_tags, summary);

		detail_slider = Slider(Slider::Type::CLASSIC, has_summary ? SHORT_DETAIL_SLIDER : DETAIL_SLIDER,
			DETAIL_X + DETAIL_SLIDER_X, 1, 1, [&](bool up) { scroll_details(detail_offset + (up ? -1 : 1)); });
		detail_offset = std::min(detail_offset, detail_steps());
		detail_slider.setrows(detail_offset, 1, detail_steps() + 1);
		summary_offset = std::min(summary_offset, summary_steps());
		summary_slider.setrows(summary_offset, 1, summary_steps() + 1);
	}

	Range<int16_t> UIQuestLog::text_view() const
	{
		return has_summary ? SHORT_TEXT_VIEW : TEXT_VIEW;
	}

	int16_t UIQuestLog::detail_steps() const
	{
		int16_t view = text_view().length() - (TEXT_POS.y() - text_view().first());

		return std::max<int16_t>(0, (description.height() - view + TEXT_STEP - 1) / TEXT_STEP);
	}

	int16_t UIQuestLog::summary_steps() const
	{
		int16_t view = SUMMARY_VIEW.length() - (SUMMARY_POS.y() - SUMMARY_VIEW.first());

		return std::max<int16_t>(0, (summary.height() - view + TEXT_STEP - 1) / TEXT_STEP);
	}

	void UIQuestLog::show_details(bool open)
	{
		details_open = open && selected >= 0;

		buttons[HIDE]->set_active(details_open);
		buttons[FIND_NPC]->set_active(details_open && tab == TAB0);
		buttons[HELPER]->set_active(details_open && tab == TAB1);
		buttons[FORFEIT]->set_active(details_open && tab == TAB1);

		if (tab == TAB1 && selected >= 0)
			buttons[HELPER]->set_state(questlog.is_tracked(selected) || questlog.tracked().size() < QuestLog::MAX_TRACKED
				? Button::NORMAL : Button::DISABLED);

		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
	}

	void UIQuestLog::scroll_list(int16_t offset)
	{
		int16_t count = static_cast<int16_t>(rows.size());
		list_offset = std::clamp<int16_t>(offset, 0, std::max<int16_t>(0, count - ROWS));
	}

	void UIQuestLog::scroll_details(int16_t offset)
	{
		detail_offset = std::clamp<int16_t>(offset, 0, detail_steps());
	}

	void UIQuestLog::scroll_summary(int16_t offset)
	{
		summary_offset = std::clamp<int16_t>(offset, 0, summary_steps());
	}

	void UIQuestLog::draw(float alpha) const
	{
		list_background.draw(position);
		tabs.draw(position + TAB_STRIP, 3, tab);

		if (rows.empty())
		{
			const Texture& notice = notices[tab];
			notice.draw(position + Point<int16_t>((LIST_WIDTH - notice.width()) / 2, 180));
		}

		for (int16_t i = 0; i < ROWS && list_offset + i < static_cast<int16_t>(rows.size()); i++)
		{
			const Row& row = rows[list_offset + i];
			Point<int16_t> top = position + Point<int16_t>(0, ROW_TOP + ROW_PITCH * i);
			Point<int16_t> label = top + Point<int16_t>(LABEL_X, LABEL_Y);

			if (row.quest < 0)
			{
				GraphicsGL::get().drawrectangle(top.x() + ROW_X, top.y(), ROW_RIGHT - ROW_X, BAR_HEIGHT,
					0.59f, 0.71f, 0.79f, 1.0f);
				fold_boxes[folded[tab].count(row.area)].draw(top + FOLD_BOX);
				row.label.draw(label);
				continue;
			}

			switch (row.icon)
			{
				case Icon::AVAILABLE:
					available_icon.draw(top + ROW_ICON);
					break;
				case Icon::IN_PROGRESS:
					progress_animation.draw(top + ARROW_ICON, alpha);
					break;
				case Icon::READY:
					ready_animation.draw(top + ROW_ICON, alpha);
					break;
				case Icon::COMPLETE:
					complete_icon.draw(top + ROW_ICON);
					break;
			}

			if (row.quest == selected && details_open)
			{
				GraphicsGL::get().drawrectangle(top.x() + LABEL_X - 2, top.y(), ROW_RIGHT - LABEL_X + 2, BAR_HEIGHT,
					0.16f, 0.37f, 0.59f, 1.0f);
				row.chosen.draw(label);
			}
			else
			{
				row.label.draw(label);
			}
		}

		list_slider.draw(position);

		if (details_open)
		{
			Point<int16_t> detail = position + Point<int16_t>(DETAIL_X, 0);

			(has_summary ? summary_background : detail_background).draw(detail);
			portrait.draw(detail + PORTRAIT_POS, alpha);
			title.draw(detail + TITLE_POS);
			jobs.draw(detail + JOBS_POS);
			levels.draw(detail + LEVELS_POS);

			Range<int16_t> clip = Range<int16_t>(detail.y() + text_view().first(), detail.y() + text_view().second());
			Point<int16_t> text = detail + TEXT_POS - Point<int16_t>(0, detail_offset * TEXT_STEP);

			images.draw(description, text, clip);
			description.draw(text, clip);
			detail_slider.draw(position);

			if (has_summary)
			{
				clip = Range<int16_t>(detail.y() + SUMMARY_VIEW.first(), detail.y() + SUMMARY_VIEW.second());
				text = detail + SUMMARY_POS - Point<int16_t>(0, summary_offset * TEXT_STEP);
				summary_images.draw(summary, text, clip);
				summary.draw(text, clip);
				summary_slider.draw(position);
			}
		}

		UIElement::draw_buttons(alpha);
	}

	void UIQuestLog::find_npc()
	{
		UIWorldMap::find_npc(start_npc);
	}

	void UIQuestLog::doubleclick(Point<int16_t> cursorpos)
	{
		if (!details_open)
			return;

		Point<int16_t> detail = position + Point<int16_t>(DETAIL_X, 0);

		// The portrait finds the NPC on the first two tabs (FUN_0087e5d0).
		Point<int16_t> corner = detail + PORTRAIT_POS - portrait.get_origin();

		if (tab != TAB2 && Rectangle<int16_t>(corner, corner + portrait.get_dimensions()).contains(cursorpos))
		{
			find_npc();
			return;
		}

		// An NPC's name in the text finds that NPC, and becomes the one
		// FIND NPC finds; a map's name shows that map on the world map. This
		// works on every tab.
		if (!text_view().contains(cursorpos.y() - detail.y()))
			return;

		Point<int16_t> text = detail + TEXT_POS - Point<int16_t>(0, detail_offset * TEXT_STEP);
		size_t index = description.index_at(cursorpos - text);

		for (const NpcTextMarkup::Link& link : links)
		{
			if (index < link.first || index >= link.last)
				continue;

			if (link.tag == 'p')
			{
				start_npc = link.id;
				find_npc();
			}
			else
			{
				UIWorldMap::find_map(link.id);
			}

			return;
		}
	}

	void UIQuestLog::close()
	{
		deactivate();
	}

	void UIQuestLog::toggle_active()
	{
		if (active)
		{
			close();
		}
		else
		{
			makeactive();
			seen_revision = std::numeric_limits<uint64_t>::max();
		}
	}

	void UIQuestLog::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
			close();
		else if (keycode == KeyAction::Id::TAB)
			change_tab((tab + 1) % 3);
	}

	void UIQuestLog::send_scroll(double yoffset)
	{
		if (details_open && has_summary && cursor_relative.x() >= DETAIL_X && cursor_relative.y() >= SUMMARY_VIEW.first() - 1)
			summary_slider.send_scroll(yoffset);
		else if (details_open && cursor_relative.x() >= DETAIL_X)
			detail_slider.send_scroll(yoffset);
		else
			list_slider.send_scroll(yoffset);
	}

	Cursor::State UIQuestLog::send_cursor(bool clicking, Point<int16_t> cursorpos)
	{
		cursor_relative = cursorpos - position;

		if (Cursor::State state = list_slider.send_cursor(cursor_relative, clicking))
			return state;

		if (details_open)
			if (Cursor::State state = detail_slider.send_cursor(cursor_relative, clicking))
				return state;

		if (details_open && has_summary)
			if (Cursor::State state = summary_slider.send_cursor(cursor_relative, clicking))
				return state;

		int16_t x = cursor_relative.x();
		int16_t y = cursor_relative.y() - ROW_TOP;
		int16_t index = y >= 0 && y % ROW_PITCH < BAR_HEIGHT ? list_offset + y / ROW_PITCH : -1;

		if (x >= ROW_X && x < ROW_RIGHT && y >= 0 && y < ROWS * ROW_PITCH && index < static_cast<int16_t>(rows.size()) && index >= 0)
		{
			if (clicking)
			{
				const Row& row = rows[index];

				if (row.quest < 0)
				{
					if (!folded[tab].erase(row.area))
						folded[tab].insert(row.area);

					refresh_list();
				}
				else
				{
					select_quest(row.quest);
				}

				return Cursor::State::CLICKING;
			}

			return Cursor::State::CANCLICK;
		}

		return UIDragElement::send_cursor(clicking, cursorpos);
	}

	Button::State UIQuestLog::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case TAB0:
			case TAB1:
			case TAB2:
				change_tab(buttonid - TAB0);
				return Button::State::IDENTITY;
			case CLOSE:
				close();
				break;
			case HIDE:
				show_details(false);
				break;
			case FIND_NPC:
				find_npc();
				break;
			case HELPER:
			{
				auto& quests = Stage::get().get_player().get_quests();

				if (quests.is_tracked(selected))
				{
					quests.untrack(selected);
				}
				else if (quests.track(selected))
				{
					// Tracking a quest brings back a closed helper.
					if (auto helper = UI::get().get_element<UIQuestHelper>())
						helper->makeactive();
				}

				break;
			}
			case FORFEIT:
				give_up(selected);
				break;
			default:
				break;
		}

		return Button::State::NORMAL;
	}

	// v83 asks first, takes the quest off the quest helper, and sends the forfeit (FUN_00a26ea7). The server's
	// record 0 then lets the quest alert offer the quest again.
	void UIQuestLog::give_up(int16_t id)
	{
		if (!questlog.started_quests().count(id))
			return;

		UI::get().emplace<UIYesNo>("Are you sure you want to quit?", [id](bool yes)
		{
			if (!yes)
				return;

			Stage::get().get_player().get_quests().untrack(id);
			QuestActionPacket(QuestActionPacket::FORFEIT, id, 0, std::nullopt).dispatch();
		});
	}

	void UIQuestLog::show_quest(int16_t id, uint16_t tabid)
	{
		int16_t area = nl::nx::quest["QuestInfo.img"][std::to_string(id)]["area"].get_integer();
		change_tab(tabid);
		bool was_folded = folded[tab].erase(area) != 0;
		selected = id;
		refresh_list();
		auto found = std::find_if(rows.begin(), rows.end(), [&](const Row& row) { return row.quest == id; });
		// Not listed: the area folds back as it was.
		if (found == rows.end() && was_folded)
		{
			folded[tab].insert(area);
			refresh_list();
		}
		if (found != rows.end())
		{
			int16_t row = static_cast<int16_t>(found - rows.begin());
			if (row < list_offset || row >= list_offset + ROWS)
				scroll_list(std::max<int16_t>(0, row - ROWS + 1));
		}
	}

	void UIQuestLog::change_tab(uint16_t tabid)
	{
		// The tab shown, clicked again, drops the pick but keeps its place
		// in the list (0x87fd57, 0x4d7dac).
		if (tabid != tab)
			list_offset = 0;

		buttons[TAB0 + tab]->set_state(Button::State::NORMAL);
		tab = tabid;
		buttons[TAB0 + tab]->set_state(Button::State::PRESSED);

		selected = -1;
		refresh_list();
	}

	UIElement::Type UIQuestLog::get_type() const
	{
		return TYPE;
	}
}
