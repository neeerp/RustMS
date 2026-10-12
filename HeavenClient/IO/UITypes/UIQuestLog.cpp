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
#include "UIWorldMap.h"
#include "../UI.h"
#include "../Components/MapleButton.h"
#include "../../Constants.h"
#include "../../Graphics/GraphicsGL.h"
#include "../../Gameplay/Stage.h"
#include "../../Gameplay/MapleMap/Npc.h"
#include "../../Gameplay/QuestConversation.h"
#include "../../Util/Misc.h"
#include <nlnx/nx.hpp>
#include <algorithm>
#include <cctype>
#include <ctime>
#include <iomanip>
#include <limits>
#include <queue>
#include <set>
#include <sstream>

namespace ms
{
	namespace
	{
		std::string lower(std::string text)
		{
			for (char& c : text) c = std::tolower(static_cast<unsigned char>(c));
			return text;
		}

		// Follow the active beginner graph, avoiding old disconnected map
		// variants that contain duplicates of these NPCs.
		int32_t npc_map(int32_t npc)
		{
			static const auto locations = [] {
				std::map<int32_t, int32_t> result;
				std::queue<int32_t> pending; pending.push(10000);
				std::set<int32_t> visited;
				while (!pending.empty())
				{
					int32_t id = pending.front(); pending.pop();
					if (!visited.insert(id).second) continue;
					auto map = nl::nx::map["Map"]["Map0"][string_format::extend_id(id, 9) + ".img"];
					for (auto life : map["life"])
						if (life["type"].get_string() == "n")
							result.emplace(string_conversion::or_default<int32_t>(life["id"].get_string(), 0), id);
					for (auto portal : map["portal"])
					{
						int32_t to = portal["tm"];
						if (to > 0 && to < 100000000 && to != id) pending.push(to);
					}
				}
				return result;
			}();
			auto found = locations.find(npc);
			return found == locations.end() ? -1 : found->second;
		}

		std::string quest_text(std::string text)
		{
			for (size_t i = 0; i < text.size(); ++i)
				if (text[i] == '\n') { text.replace(i, 1, "\\n"); ++i; }
			// Resolve the name/map placeholders used in quest descriptions.
			for (size_t start = 0; (start = text.find('#', start)) != std::string::npos;)
			{
				size_t end = text.find('#', start + 2);
				if (end == std::string::npos) break;
				char kind = start + 1 < text.size() ? text[start + 1] : 0;
				if (kind != 'p' && kind != 'm' && kind != 'h') { ++start; continue; }
				std::string replacement;
				if (kind == 'h') replacement = Stage::get().get_player().get_name();
				else
				{
					auto id = string_conversion::or_default<int32_t>(text.substr(start + 2, end - start - 2), -1);
					if (id < 0) { ++start; continue; }
					replacement = kind == 'p' ? nl::nx::string["Npc.img"][std::to_string(id)]["name"].get_string()
						: NxHelper::Map::get_map_info_by_id(id).name;
				}
				text.replace(start, end - start + 1, replacement);
				start += replacement.size();
			}
			return text;
		}
	}

	UIQuestLog::UIQuestLog(const QuestLog& ql) : UIDragElement<PosQUEST>(), questlog(ql)
	{
		tab = Buttons::TAB0;
		auto quest = nl::nx::ui["UIWindow2.img"]["Quest"];
		auto list = quest["list"];
		auto info = quest["quest_info"];
		sprites.emplace_back(list["backgrnd"]);
		sprites.emplace_back(list["backgrnd2"]);
		for (int i = 0; i < 3; ++i)
		{
			notice_sprites.emplace_back(list["notice" + std::to_string(i)]);
			buttons[TAB0 + i] = std::make_unique<TwoSpriteButton>(list["Tab"]["disabled"][std::to_string(i)],
				list["Tab"]["enabled"][std::to_string(i)]);
			if (i > 0) quest_icons[i] = quest["icon"][i == 1 ? "icon2" : "icon3"]["0"];
		}
		// The available icon is a bitmap; the others are animations' first frames.
		quest_icons[0] = quest["icon"]["icon0"];
		buttons[RECOMMENDED] = std::make_unique<TwoSpriteButton>(list["Tab"]["disabled"]["3"], list["Tab"]["enabled"]["3"]);
		buttons[RECOMMENDED]->set_state(Button::DISABLED);
		buttons[CLOSE] = std::make_unique<MapleButton>(nl::nx::ui["Basic.img"]["BtClose3"], Point<int16_t>(275, 6));
		buttons[SEARCH] = std::make_unique<MapleButton>(list["BtSearch"]);
		buttons[ALL_LEVEL] = std::make_unique<MapleButton>(list["BtAllLevel"]);
		buttons[MY_LEVEL] = std::make_unique<MapleButton>(list["BtMyLevel"]);
		buttons[MY_LOCATION] = std::make_unique<MapleButton>(list["BtMyLocation"]);
		buttons[ALL_LOCATION] = std::make_unique<MapleButton>(list["BtAllLocation"]);
		buttons[DETAIL_CLOSE] = std::make_unique<MapleButton>(info["BtClose"], Point<int16_t>(DETAIL_X, 0));
		buttons[ACCEPT] = std::make_unique<MapleButton>(info["BtAccept"], Point<int16_t>(DETAIL_X, 0));
		buttons[FINISH] = std::make_unique<MapleButton>(info["BtFinish"], Point<int16_t>(DETAIL_X, 0));
		buttons[FIND_NPC] = std::make_unique<MapleButton>(info["BtNPC"], Point<int16_t>(DETAIL_X, 0));
		buttons[HELPER] = std::make_unique<MapleButton>(info["BtArlim"], Point<int16_t>(DETAIL_X, 0));
		buttons[FORFEIT] = std::make_unique<MapleButton>(info["BtGiveup"], Point<int16_t>(DETAIL_X, 0));
		info_background = info["backgrnd"]; info_paper = info["backgrnd2"];
		info_header = info["backgrnd3"]; info_tip = info["tip"];
		reward_background = info["summary"]; reward_background.shift(reward_background.get_origin());
		reward_heading = info["summary_icon"]["reward"];
		basic_reward = info["summary_icon"]["basic"];
		search_area = list["searchArea"]; complete_count = list["completeCount"];
		// Textfield bounds are relative to the movable window, never screen coordinates.
		search = Textfield(Text::A11M, Text::LEFT, Color::Name::DARKGREY,
			Rectangle<int16_t>({36, 49}, {235, 71}), 40);
		search.set_enter_callback([&](std::string) { search.set_state(Textfield::DISABLED); search.set_state(Textfield::NORMAL); refresh_list(); });
		search.set_key_callback(KeyAction::ESCAPE, [&] { close(); });
		placeholder = Text(Text::A11M, Text::LEFT, Color::Name::BOULDER, "Enter the quest name.");
		empty_detail = Text(Text::A12M, Text::CENTER, Color::Name::BOULDER, "Select a quest to view its information.", 250);
		slider = Slider(Slider::DEFAULT_SILVER, {78, 353}, 278, VISIBLE_ROWS, 0, [&](bool up) { list_offset += up ? -1 : 1; });
		detail_slider = Slider(Slider::DEFAULT_SILVER, {130, 353}, DETAIL_X + 278, 235, 235,
			[&](bool up) { detail_offset = std::clamp<int16_t>(detail_offset + (up ? -16 : 16), 0, std::max<int16_t>(0, detail_height - 235)); });
		seen_revision = std::numeric_limits<uint64_t>::max();
		change_tab(TAB0);
		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
		update();
	}

	void UIQuestLog::update_screen(int16_t width, int16_t height)
	{
		dimension = {static_cast<int16_t>(detail_open ? DETAIL_X + 296 : 295), 396};
		dragarea = {dimension.x(), 20};
		if (position.x() < 0 || position.x() + dimension.x() > width)
			position.set_x(std::max<int16_t>(0, (width - dimension.x()) / 2));
		if (position.y() < 0 || position.y() + dimension.y() > height)
			position.set_y(std::max<int16_t>(0, (height - dimension.y()) / 2));
	}

	void UIQuestLog::update()
	{
		UIElement::update();
		search.update(position);
		if (selected >= 0) portrait.update();
		auto& player = Stage::get().get_player();
		auto job = player.get_stats().get_stat(MapleStat::JOB);
		auto level = player.get_stats().get_stat(MapleStat::LEVEL);
		auto map = Stage::get().get_mapid();
		if (seen_revision != questlog.revision() || seen_job != job || seen_level != level || seen_map != map || seen_search != search.get_text())
		{
			seen_revision = questlog.revision(); seen_job = job; seen_level = level; seen_map = map; seen_search = search.get_text();
			refresh_list();
		}
		bool local = selected >= 0 && local_npc_oid() >= 0;
		auto enable = [&](Buttons id, bool allowed) {
			if (!allowed) buttons[id]->set_state(Button::DISABLED);
			else if (buttons[id]->get_state() == Button::DISABLED) buttons[id]->set_state(Button::NORMAL);
		};
		enable(ACCEPT, local && questlog.npc_marker(target_npc, job, level) == QuestMarker::AVAILABLE);
		enable(FINISH, local && questlog.npc_marker(target_npc, job, level) == QuestMarker::READY);
	}

	void UIQuestLog::refresh_list()
	{
		std::vector<int16_t> ids;
		auto& stats = Stage::get().get_player().get_stats();
		auto eligible = questlog.available_quests(stats.get_stat(MapleStat::JOB), stats.get_stat(MapleStat::LEVEL));
		if (tab == TAB0) ids = questlog.available_quests(stats.get_stat(MapleStat::JOB), stats.get_stat(MapleStat::LEVEL), all_levels);
		else if (tab == TAB1) for (const auto& q : questlog.started_quests()) ids.push_back(q.first);
		else for (const auto& q : questlog.completed_quests()) ids.push_back(q.first);
		rows.clear();
		std::string filter = lower(search.get_text());
		for (int16_t id : ids)
		{
			auto info = nl::nx::quest["QuestInfo.img"][std::to_string(id)];
			std::string name = info["name"];
			if (name.empty() || (tab != TAB2 && lower(name).find(filter) == std::string::npos)) continue;
			int32_t npc = nl::nx::quest["Check.img"][std::to_string(id)][tab == TAB0 ? "0" : "1"]["npc"];
			if (current_location && tab == TAB0)
			{
				bool here = false;
				for (const auto& object : *Stage::get().get_npcs().get_npcs())
					if (auto n = dynamic_cast<Npc*>(object.second.get())) if (n->get_npcid() == npc) here = true;
				if (!here) continue;
			}
			auto check = nl::nx::quest["Check.img"][std::to_string(id)]["0"];
			int16_t minimum = check["lvmin"] ? check["lvmin"].get_integer() : 1;
			std::string label = "(Lv. " + std::to_string(minimum) + ") " + name;
			Text title(Text::A11M, Text::LEFT, Color::Name::DARKGREY, label, 0, false);
			while (title.width() > 235 && label.size() > 4)
			{
				label.pop_back(); title.change_text(label + "...");
			}
			rows.push_back({id, std::move(title), tab != TAB0 || std::find(eligible.begin(), eligible.end(), id) != eligible.end()});
		}
		group_title = Text(Text::A11B, Text::LEFT, Color::Name::WHITE, "Maple Island (" + std::to_string(rows.size()) + ")", 0, false);
		completion_count = Text(Text::A11B, Text::RIGHT, Color::Name::WHITE, std::to_string(questlog.completed_quests().size()));
		int16_t total = rows.empty() ? 0 : collapsed ? 1 : rows.size() + 1;
		list_offset = std::clamp<int16_t>(list_offset, 0, std::max<int16_t>(0, total - VISIBLE_ROWS));
		slider.setrows(list_offset, VISIBLE_ROWS, total);
		auto found = std::find_if(rows.begin(), rows.end(), [&](const auto& q) { return q.id == selected; });
		if (found == rows.end()) selected = rows.empty() ? -1 : rows.front().id;
		select_quest(selected);
	}

	void UIQuestLog::select_quest(int16_t id)
	{
		selected = id; detail_offset = 0;
		for (auto& row : rows) row.title.change_color(row.id == id ? Color::Name::WHITE : row.eligible ? Color::Name::DARKGREY : Color::Name::BOULDER);
		buttons[DETAIL_CLOSE]->set_active(detail_open);
		for (auto b : {ACCEPT, FINISH, FIND_NPC, HELPER, FORFEIT}) buttons[b]->set_active(detail_open && selected >= 0);
		buttons[ACCEPT]->set_active(detail_open && selected >= 0 && tab == TAB0);
		buttons[FINISH]->set_active(detail_open && selected >= 0 && tab == TAB1);
		buttons[HELPER]->set_active(detail_open && selected >= 0 && tab == TAB1);
		buttons[FORFEIT]->set_active(detail_open && selected >= 0 && tab == TAB1);
		buttons[FIND_NPC]->set_active(detail_open && selected >= 0 && tab != TAB1);
		buttons[HELPER]->set_state(Button::DISABLED); buttons[FORFEIT]->set_state(Button::DISABLED);
		if (id < 0) { target_npc = 0; target_map = -1; return; }
		auto info = nl::nx::quest["QuestInfo.img"][std::to_string(id)];
		auto check = nl::nx::quest["Check.img"][std::to_string(id)];
		target_npc = check[tab == TAB0 ? "0" : "1"]["npc"];
		target_map = npc_map(target_npc);
		heading = Text(Text::A12B, Text::LEFT, Color::Name::WHITE, info["name"], 172);
		int16_t minimum = check["0"]["lvmin"] ? check["0"]["lvmin"].get_integer() : 1;
		std::string requirement = "Level " + std::to_string(minimum);
		if (check["0"]["lvmax"]) requirement += " - " + std::to_string(check["0"]["lvmax"].get_integer());
		else requirement += "+";
		level_text = Text(Text::A11M, Text::LEFT, Color::Name::WHITE, requirement);
		portrait = nl::nx::npc[string_format::extend_id(target_npc, 7) + ".img"]["stand"];
		std::string npc_name = nl::nx::string["Npc.img"][std::to_string(target_npc)]["name"];
		objective = Text(Text::A12B, Text::LEFT, Color::Name::BLUE, "Talk to " + npc_name, 250);
		auto map_name = target_map >= 0 ? NxHelper::Map::get_map_info_by_id(target_map).name : "";
		location = Text(Text::A11B, Text::LEFT, Color::Name::RED, map_name, 250);
		std::string text = info[std::to_string(tab)].get_string();
		if (text.empty()) text = info["1"].get_string();
		description = Text(Text::A12M, Text::LEFT, Color::Name::DARKGREY, quest_text(text), 250);
		reward_y = std::max<int16_t>(245, 180 + description.height() + 24);
		auto experience = nl::nx::quest["Act.img"][std::to_string(id)]["1"]["exp"].get_integer();
		reward = Text(Text::A12M, Text::LEFT, Color::Name::DARKGREY, std::to_string(experience) + " EXP");
		completed_on = Text();
		if (tab == TAB2)
		{
			// Quest packets store Windows FILETIME ticks.
			int64_t ticks = questlog.completed_quests().at(id);
			std::time_t seconds = ticks / 10000000 - 11644473600LL;
			std::tm* date = std::localtime(&seconds);
			if (date)
			{
				std::ostringstream stamp; stamp << "Completed on " << std::put_time(date, "%Y-%m-%d");
				completed_on = Text(Text::A11B, Text::LEFT, Color::Name::BLUE, stamp.str());
			}
		}
		detail_height = reward_y + 112 - 130;
		detail_slider.setrows(1, 1 + (std::max<int16_t>(0, detail_height - 235) + 15) / 16);
		buttons[FIND_NPC]->set_state(target_map >= 0 ? Button::NORMAL : Button::DISABLED);
	}

	void UIQuestLog::draw(float alpha) const
	{
		draw_sprites(alpha);
		if (tab == TAB2)
		{
			complete_count.draw(position);
			completion_count.draw(position + Point<int16_t>(272, 44));
		}
		else
		{
			search_area.draw(position); search.draw(position);
			if (search.empty() && search.get_state() != Textfield::FOCUSED) placeholder.draw(position + Point<int16_t>(39, 50));
		}
		if (rows.empty()) notice_sprites[tab].draw(position, alpha);
		for (int16_t row = 0; row < VISIBLE_ROWS && !rows.empty(); ++row)
		{
			int16_t index = row + list_offset;
			auto pos = position + Point<int16_t>(12, ROW_Y + row * ROW_HEIGHT + 9);
			if (index == 0)
			{
				GraphicsGL::get().drawrectangle(pos.x(), pos.y() - 9, 263, 18, .21f, .43f, .61f, 1.0f);
				Text(Text::A11B, Text::LEFT, Color::Name::WHITE, collapsed ? "+" : "-").draw(pos + Point<int16_t>(3, -15));
				group_title.draw(pos + Point<int16_t>(17, -15));
			}
			else if (!collapsed && index <= static_cast<int16_t>(rows.size()))
			{
				const auto& q = rows[index - 1];
				bool chosen = q.id == selected;
				if (chosen) GraphicsGL::get().drawrectangle(pos.x(), pos.y() - 9, 263, 18, .60f, .60f, .60f, 1.0f);
				quest_icons[tab].draw(pos + Point<int16_t>(0, -7));
				q.title.draw(pos + Point<int16_t>(17, -15));
			}
			else break;
		}
		slider.draw(position);
		if (detail_open)
		{
			auto origin = position + Point<int16_t>(DETAIL_X, 0);
			info_background.draw(origin); info_paper.draw(origin);
			if (selected >= 0)
			{
				info_header.draw(origin); info_tip.draw(origin);
				portrait.draw(origin + Point<int16_t>(234, 119), alpha);
				heading.draw(origin + Point<int16_t>(20, 43));
				level_text.draw(origin + Point<int16_t>(20, 43 + std::max<int16_t>(20, heading.height()) + 8));
				Range<int16_t> clip(position.y() + 130, position.y() + 364);
				auto content = origin - Point<int16_t>(0, detail_offset);
				(tab == TAB2 ? completed_on : objective).draw(content + Point<int16_t>(16, 140), clip);
				location.draw(content + Point<int16_t>(16, 160), clip);
				description.draw(content + Point<int16_t>(16, 180), clip);
				// Keep the reward panel inside the detail viewport while scrolling.
				int16_t top = std::max<int16_t>(130, reward_y - detail_offset);
				int16_t bottom = std::min<int16_t>(364, reward_y + 112 - detail_offset);
				if (bottom > top) reward_background.draw(DrawArgument(origin + Point<int16_t>(10, top), Point<int16_t>(263, bottom - top)));
				auto icon = [&](const Texture& texture, int16_t y) {
					if (y - detail_offset >= 130 && y - detail_offset + texture.height() <= 364)
						texture.draw(content + Point<int16_t>(20, y));
				};
				icon(reward_heading, reward_y + 9); icon(basic_reward, reward_y + 36);
				reward.draw(content + Point<int16_t>(26, reward_y + 64), clip);
				detail_slider.draw(position);
			}
			else empty_detail.draw(origin + Point<int16_t>(148, 195));
		}
		draw_buttons(alpha);
	}

	int32_t UIQuestLog::local_npc_oid() const
	{
		for (const auto& object : *Stage::get().get_npcs().get_npcs())
			if (auto n = dynamic_cast<Npc*>(object.second.get()))
				if (n->get_npcid() == target_npc) return object.first;
		return -1;
	}

	void UIQuestLog::find_npc()
	{
		if (target_map < 0) return;
		search.set_state(Textfield::DISABLED);
		auto map = UI::get().emplace<UIWorldMap>();
		if (map) map->show_location(target_map);
		search.set_state(tab == TAB2 ? Textfield::DISABLED : Textfield::NORMAL);
	}

	void UIQuestLog::doubleclick(Point<int16_t> pos)
	{
		if (detail_open && selected >= 0 && Rectangle<int16_t>(position + Point<int16_t>(DETAIL_X + 185, 27), position + Point<int16_t>(DETAIL_X + 280, 122)).contains(pos)) find_npc();
	}

	void UIQuestLog::close() { search.set_state(Textfield::DISABLED); deactivate(); }
	void UIQuestLog::toggle_active()
	{
		if (active) close();
		else { makeactive(); search.set_state(tab == TAB2 ? Textfield::DISABLED : Textfield::NORMAL); seen_revision = std::numeric_limits<uint64_t>::max(); }
	}

	void UIQuestLog::send_key(int32_t key, bool pressed, bool escape)
	{
		if (!pressed) return;
		if (escape) close();
		else if (key == KeyAction::TAB) change_tab((tab + 1) % 3);
	}

	void UIQuestLog::send_scroll(double offset)
	{
		if (detail_open && cursor_relative.x() >= DETAIL_X) detail_slider.send_scroll(offset);
		else slider.send_scroll(offset);
	}

	Cursor::State UIQuestLog::send_cursor(bool clicking, Point<int16_t> pos)
	{
		cursor_relative = pos - position;
		if (!clicking) click_down = false;
		if (clicking && search.get_state() == Textfield::FOCUSED && !search.get_bounds().contains(pos))
		{ search.set_state(Textfield::DISABLED); search.set_state(tab == TAB2 ? Textfield::DISABLED : Textfield::NORMAL); }
		if (auto state = search.send_cursor(pos, clicking)) return state;
		if (!collapsed && rows.size() + 1 > VISIBLE_ROWS) if (auto state = slider.send_cursor(cursor_relative, clicking)) return state;
		if (detail_open && selected >= 0 && detail_height > 235) if (auto state = detail_slider.send_cursor(cursor_relative, clicking)) return state;
		if (cursor_relative.x() >= 12 && cursor_relative.x() <= 275 && cursor_relative.y() >= ROW_Y && cursor_relative.y() < ROW_Y + VISIBLE_ROWS * ROW_HEIGHT && !rows.empty())
		{
			if (clicking && !click_down)
			{
				click_down = true;
				int16_t row = (cursor_relative.y() - ROW_Y) / ROW_HEIGHT + list_offset;
				if (row == 0) { collapsed = !collapsed; refresh_list(); }
				else if (!collapsed && row <= static_cast<int16_t>(rows.size()))
				{
					detail_open = true; select_quest(rows[row - 1].id);
					update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
				}
			}
			return Cursor::CANCLICK;
		}
		return UIDragElement::send_cursor(clicking, pos);
	}

	Button::State UIQuestLog::button_pressed(uint16_t id)
	{
		if (id <= TAB2) { change_tab(id); return Button::IDENTITY; }
		switch (id)
		{
			case CLOSE: close(); break;
			case DETAIL_CLOSE:
				detail_open = false; select_quest(selected);
				update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight()); break;
			case SEARCH: refresh_list(); break;
			case ALL_LEVEL:
			case MY_LEVEL:
				all_levels = !all_levels;
				buttons[ALL_LEVEL]->set_active(!all_levels); buttons[MY_LEVEL]->set_active(all_levels);
				refresh_list(); break;
			case MY_LOCATION:
			case ALL_LOCATION:
				current_location = !current_location;
				buttons[MY_LOCATION]->set_active(!current_location); buttons[ALL_LOCATION]->set_active(current_location);
				refresh_list(); break;
			case FIND_NPC: find_npc(); break;
			case ACCEPT:
			case FINISH:
				if (local_npc_oid() >= 0) { QuestConversation::open(selected, target_npc); close(); } break;
			default: break;
		}
		return Button::NORMAL;
	}

	void UIQuestLog::change_tab(uint16_t id)
	{
		buttons[TAB0 + tab]->set_state(Button::NORMAL);
		tab = id; selected = -1; list_offset = 0; collapsed = false;
		search.set_state(Textfield::DISABLED); search.change_text("");
		search.set_state(tab == TAB2 ? Textfield::DISABLED : Textfield::NORMAL);
		buttons[MY_LOCATION]->set_active(tab == TAB0 && !current_location);
		buttons[ALL_LOCATION]->set_active(tab == TAB0 && current_location);
		buttons[ALL_LEVEL]->set_active(tab == TAB0 && !all_levels);
		buttons[MY_LEVEL]->set_active(tab == TAB0 && all_levels);
		buttons[SEARCH]->set_active(tab != TAB2);
		buttons[TAB0 + tab]->set_state(Button::PRESSED);
		refresh_list();
	}

	UIElement::Type UIQuestLog::get_type() const { return TYPE; }
}
