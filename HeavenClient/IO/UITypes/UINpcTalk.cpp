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
#include "UINpcTalk.h"

#include "../KeyAction.h"
#include "../UI.h"

#include "../Components/MapleButton.h"
#include "../Gameplay/Stage.h"

#include "../Net/Packets/NpcInteractionPackets.h"
#include "../../Constants.h"
#include "../../Data/ItemData.h"
#include "../../Data/SkillData.h"
#include "../../Graphics/Geometry.h"
#include "../../Util/Misc.h"
#include "../../Util/NpcTextMarkup.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	static_assert(NpcTextMarkup::IMAGE == Text::BOX, "dialogue images must be laid out as boxes");

	namespace
	{
		// The window, measured from period footage. The middle stretches to
		// fit the text plus 34 px, and the window is centred on the screen.
		constexpr int16_t TOP = 28;
		constexpr int16_t TEXT_X = 165;
		constexpr int16_t TEXT_WIDTH = 312;
		constexpr int16_t LINE = 18;
		constexpr int16_t PADDING = 34;
		// Lines shown at once when the text scrolls, and where they start.
		constexpr int16_t SCROLL_LINES = 11;
		constexpr int16_t SCROLL_TEXT_Y = 28;
		// A choice's dot and label, from where the choice starts.
		constexpr int16_t DOT_X = 10;
		constexpr int16_t LABEL_X = 18;
		// Listed choices: where they start under the question, the gap
		// before each later header, a header's top, and from a header to
		// its first row.
		constexpr int16_t LIST_TOP = 25;
		constexpr int16_t HEADER_GAP = 9;
		constexpr int16_t HEADER_Y = 4;
		constexpr int16_t HEADER_ROW = 11;
		constexpr int16_t ROW = 32;
		// How long each character takes to type out: footage types about 35
		// a second.
		constexpr uint16_t TYPE_MS = 28;

		// An image by path, such as UI/UIWindow.img/QuestIcon/4/0. UI paths
		// name the v83 UI; our UI.nx has the quest icons under UIWindow2.img.
		Texture path_image(const std::string &path)
		{
			size_t slash = path.find('/');

			if (slash == std::string::npos)
				return Texture();

			const std::pair<std::string, nl::node> files[] = {
				{ "UI", NxFiles::ui_classic }, { "Map", nl::nx::map }, { "Effect", nl::nx::effect },
				{ "Item", nl::nx::item }, { "Skill", nl::nx::skill }, { "Mob", nl::nx::mob }, { "Npc", nl::nx::npc },
				{ "Character", nl::nx::character }, { "Etc", nl::nx::etc }, { "Quest", nl::nx::quest },
				{ "Reactor", nl::nx::reactor }
			};

			std::string file = path.substr(0, slash);
			std::string rest = path.substr(slash + 1);

			for (const auto &[name, root] : files)
			{
				if (name != file)
					continue;

				nl::node found = root.resolve(rest);

				if (found.data_type() != nl::node::type::bitmap && name == "UI")
				{
					found = nl::nx::ui.resolve(rest);

					if (found.data_type() != nl::node::type::bitmap && rest.rfind("UIWindow.img/", 0) == 0)
						found = nl::nx::ui.resolve("UIWindow2.img/" + rest.substr(13));
				}

				return Texture(found);
			}

			return Texture();
		}

		// The dotted line under the selected choice.
		void dotted_line(Point<int16_t> from, int16_t to, Color::Name color)
		{
			ColorBox dot(1, 1, color, 1.0f);

			for (int16_t x = from.x(); x < to; x += 2)
				dot.draw(Point<int16_t>(x, from.y()));
		}
	}

	UINpcTalk::UINpcTalk() : height(MIN_HEIGHT), text_y(0), offset(0), rowmax(0), show_slider(false), draw_text(false),
							 type(TalkType::NONE), menu_kind(Menu::INLINE), message_type(0), selected(0), focus(answer_focus::Focus::NONE), formatted_text(""),
							 formatted_text_pos(0), timestep(0)
	{
		nl::node UtilDlgEx = NxFiles::ui_classic["UIWindow.img"]["UtilDlgEx"];

		top = UtilDlgEx["t"];
		fill = UtilDlgEx["c"];
		bottom = UtilDlgEx["s"];
		nametag = UtilDlgEx["bar"];
		dot_selected = UtilDlgEx["dot1"];
		dot = UtilDlgEx["dot0"];

		buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(UtilDlgEx["BtClose"]);
		buttons[Buttons::NEXT] = std::make_unique<MapleButton>(UtilDlgEx["BtNext"]);
		buttons[Buttons::NO] = std::make_unique<MapleButton>(UtilDlgEx["BtNo"]);
		buttons[Buttons::OK] = std::make_unique<MapleButton>(UtilDlgEx["BtOK"]);
		buttons[Buttons::PREV] = std::make_unique<MapleButton>(UtilDlgEx["BtPrev"]);
		buttons[Buttons::QNO] = std::make_unique<MapleButton>(UtilDlgEx["BtQNo"]);
		buttons[Buttons::QYES] = std::make_unique<MapleButton>(UtilDlgEx["BtQYes"]);
		buttons[Buttons::YES] = std::make_unique<MapleButton>(UtilDlgEx["BtYes"]);

		name = Text(Text::Font::A13M, Text::Alignment::CENTER, Color::Name::WHITE);

		onmoved = [&](bool upwards)
		{
			int16_t shift = upwards ? -1 : 1;

			if (offset + shift >= 0 && offset + shift <= rowmax - SCROLL_LINES)
				offset += shift;
		};

		UI::get().remove_textfield();
	}

	void UINpcTalk::draw(float inter) const
	{
		// The middle repeats; stretching it would smear its dithering.
		top.draw(position);

		for (int16_t y = 0; y < height; y += fill.height())
			fill.draw(position + Point<int16_t>(0, TOP + y),
					  Rectangle<int16_t>(0, fill.width(), 0, std::min<int16_t>(fill.height(), height - y)));

		bottom.draw(position + Point<int16_t>(0, TOP + height));

		UIElement::draw(inter);

		int16_t total = TOP + height + bottom.height();
		Point<int16_t> speaker_pos = position + Point<int16_t>(20, total / 2 + 11);
		Point<int16_t> center_pos = speaker_pos + Point<int16_t>(nametag.width() / 2, 0);

		speaker.draw(DrawArgument(center_pos, true));
		nametag.draw(speaker_pos);
		name.draw(center_pos + Point<int16_t>(0, -4));

		Point<int16_t> origin = position + text_origin();
		Range<int16_t> rows = visible_rows();
		Range<int16_t> clip = show_slider ? Range<int16_t>(position.y() + rows.first(), position.y() + rows.second())
										  : Range<int16_t>(0, 0);
		auto visible = [&](int16_t top_y, int16_t bottom_y)
		{
			return !show_slider || (top_y >= clip.first() && bottom_y <= clip.second());
		};

		// The text types out, but its images show at once, as in the original.
		for (const auto &image : images)
		{
			Point<int16_t> box = origin + text.box_position(image.box);

			if (visible(box.y(), box.y() + image.size.y()))
				image.texture.draw(box + image.anchor);
		}

		text.draw(origin, clip, revealed(), instant_text);

		for (size_t i = 0; i < choices.size(); i++)
		{
			const Choice &choice = choices[i];

			if (choice.header.is_valid() && visible(origin.y() + choice.header_y,
													 origin.y() + choice.header_y + choice.header.height()))
				choice.header.draw(origin + Point<int16_t>(0, choice.header_y));

			Point<int16_t> start = origin + choice.start;

			if (!visible(start.y() - LINE + 4, start.y() + 4))
				continue;

			(i == selected ? dot_selected : dot).draw(start + Point<int16_t>(DOT_X, -8));
			choice.label.draw(start + Point<int16_t>(LABEL_X, -LINE));

			if (i != selected)
				continue;

			// A label that wraps is underlined on its first and last lines.
			Point<int16_t> end = origin + choice.end;
			Color::Name color = choice.label.empty() ? Color::Name::BLUE : Color::Name::DUSKBLUE;

			if (end.y() == start.y())
			{
				dotted_line(start + Point<int16_t>(LABEL_X, 2), end.x(), color);
			} else
			{
				dotted_line(start + Point<int16_t>(LABEL_X, 2), origin.x() + text.width(), color);

				if (visible(end.y() - LINE + 4, end.y() + 4))
					dotted_line(Point<int16_t>(origin.x(), end.y() + 2), end.x(), color);
			}
		}

		if (show_slider)
			slider.draw(position);
	}

	void UINpcTalk::send_scroll(double yoffset)
	{
		if (show_slider && slider.isenabled())
			slider.send_scroll(yoffset);
	}

	Point<int16_t> UINpcTalk::text_origin() const
	{
		return Point<int16_t>(TEXT_X, text_y - offset * LINE);
	}

	Range<int16_t> UINpcTalk::visible_rows() const
	{
		// Room below the last line for its descenders.
		return Range<int16_t>(text_y, text_y + SCROLL_LINES * LINE + 5);
	}

	size_t UINpcTalk::revealed() const
	{
		return draw_text ? formatted_text_pos : formatted_text.size();
	}

	int16_t UINpcTalk::choice_at(Point<int16_t> cursor) const
	{
		Point<int16_t> origin = text_origin();
		Range<int16_t> rows = visible_rows();

		if (show_slider && (cursor.y() < rows.first() || cursor.y() > rows.second()))
			return -1;

		for (size_t i = 0; i < choices.size(); i++)
		{
			const Choice &choice = choices[i];
			Point<int16_t> start = origin + choice.start;
			Point<int16_t> end = origin + choice.end;
			int16_t right = end.y() == start.y() ? end.x() : origin.x() + text.width();

			if (Rectangle<int16_t>(start.x(), right, start.y() - LINE + 4, start.y() + 4).contains(cursor))
				return static_cast<int16_t>(i);

			if (end.y() != start.y()
				&& Rectangle<int16_t>(origin.x(), end.x(), start.y() + 4, end.y() + 4).contains(cursor))
				return static_cast<int16_t>(i);
		}

		return -1;
	}

	void UINpcTalk::update()
	{
		UIElement::update();

		if (draw_text)
		{
			timestep += Constants::TIMESTEP;

			for (; timestep >= TYPE_MS && formatted_text_pos < formatted_text.size(); timestep -= TYPE_MS)
			{
				formatted_text_pos++;

				// What already shows is not typed again.
				for (const auto &range : instant_text)
					if (formatted_text_pos >= range.first && formatted_text_pos < range.second)
						formatted_text_pos = range.second;
			}

			if (formatted_text_pos >= formatted_text.size())
				draw_text = false;
		}
	}

	Button::State UINpcTalk::button_pressed(uint16_t buttonid)
	{
		deactivate();

		switch (type)
		{
			case TalkType::SENDNEXT:
			case TalkType::SENDOK:
			case TalkType::SENDNEXTPREV:
				// Type = 0
				switch (buttonid)
				{
					case Buttons::CLOSE:
						respond(-1);
						break;
					case Buttons::NEXT:
					case Buttons::OK:
						respond(1);
						break;
					case Buttons::PREV:
						respond(0);
						break;
				}
				break;
			case TalkType::SENDYESNO:
				// Type = 1
				switch (buttonid)
				{
					case Buttons::CLOSE:
						respond(-1);
						break;
					case Buttons::NO:
						respond(0);
						break;
					case Buttons::YES:
						respond(1);
						break;
				}
				break;
			case TalkType::SENDACCEPTDECLINE:
				// Type = 12
				switch (buttonid)
				{
					case Buttons::CLOSE:
						respond(-1);
						break;
					case Buttons::QNO:
						respond(0);
						break;
					case Buttons::QYES:
						respond(1);
						break;
				}
				break;
			case TalkType::SENDGETTEXT:
				// TODO: What is this?
				break;
			case TalkType::SENDGETNUMBER:
				// Type = 3
				switch (buttonid)
				{
					case Buttons::CLOSE:
						respond(0);
						break;
					case Buttons::OK:
						respond(1);
						break;
				}
				break;
			case TalkType::SENDSIMPLE:
				// Type = 4: NEXT answers with the selected choice.
				if (buttonid == Buttons::NEXT)
					answer_choice();
				else if (buttonid == Buttons::CLOSE)
					local_responder ? respond(-1) : respond(0);
				break;
			default:
				break;
		}

		return Button::State::NORMAL;
	}

	void UINpcTalk::respond(int8_t response, int32_t selection)
	{
		if (local_responder)
		{
			// The responder may replace this dialogue, so call a copy.
			auto responder = local_responder;
			responder(response, selection);
			return;
		}

		if (selection >= 0)
			NpcTalkMorePacket(selection).dispatch();
		else
			NpcTalkMorePacket(message_type, response).dispatch();
	}

	void UINpcTalk::select(size_t choice)
	{
		if (choice >= choices.size())
			return;

		selected = choice;

		if (!show_slider)
			return;

		// The lines the choice's label takes up.
		int16_t first = (choices[choice].start.y() - LINE) / LINE;
		int16_t last = (choices[choice].end.y() - LINE) / LINE;

		if (first < offset)
			offset = first;
		else if (last >= offset + SCROLL_LINES)
			offset = std::min<int16_t>(last - SCROLL_LINES + 1, rowmax - SCROLL_LINES);

		slider.setrows(offset, SCROLL_LINES, rowmax);
	}

	void UINpcTalk::answer_choice()
	{
		if (selected >= choices.size())
			return;

		deactivate();
		respond(1, choices[selected].id);
	}

	void UINpcTalk::set_local_responder(std::function<void(int8_t, int32_t)> responder)
	{
		local_responder = std::move(responder);
	}

	Cursor::State UINpcTalk::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Point<int16_t> cursor_relative = cursorpos - position;

		if (show_slider && slider.isenabled())
			if (Cursor::State sstate = slider.send_cursor(cursor_relative, clicked))
				return sstate;

		// Clicking a choice answers with it.
		int16_t choice = choice_at(cursor_relative);

		if (choice >= 0)
		{
			if (clicked)
			{
				selected = choice;
				answer_choice();
			}

			return Cursor::State::CANCLICK;
		}

		if (clicked && draw_text && Rectangle<int16_t>(TEXT_X, TEXT_X + TEXT_WIDTH, TOP, TOP + height).contains(cursor_relative))
		{
			draw_text = false;
			return Cursor::State::IDLE;
		}

		Cursor::State estate = UIElement::send_cursor(clicked, cursorpos);

		if (estate == Cursor::State::CLICKING && clicked && draw_text)
			draw_text = false;

		return estate;
	}

	void UINpcTalk::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
		{
			deactivate();

			respond(-1);
			return;
		}

		// The NPC chat key advances the conversation it started, as Enter
		// does.
		bool confirm = keycode == KeyAction::Id::RETURN || keycode == KeyAction::Id::INTERACT_HARVEST;

		if (type == TalkType::SENDSIMPLE)
		{
			// Up and Down move the selection; Enter answers with it.
			if (keycode == KeyAction::Id::UP && selected > 0)
				select(selected - 1);
			else if (keycode == KeyAction::Id::DOWN)
				select(selected + 1);
			else if (confirm)
				answer_choice();

			return;
		}

		// Left and Right focus YES/NO or ACCEPT/DECLINE, and Enter presses
		// the focused one, or the left one.
		Buttons left, right;

		if (answers(left, right))
		{
			if (keycode == KeyAction::Id::LEFT || keycode == KeyAction::Id::RIGHT)
				focus_answer(answer_focus::move(focus, keycode == KeyAction::Id::LEFT, keycode == KeyAction::Id::RIGHT));
			else if (confirm)
				button_pressed(answer_focus::pressed(focus) == answer_focus::Focus::LEFT ? left : right);

			return;
		}

		// Enter presses the page's default button, even while the text
		// types out.
		if (!confirm)
			return;

		for (Buttons button : { Buttons::NEXT, Buttons::OK, Buttons::YES, Buttons::QYES })
		{
			if (buttons[button]->is_active())
			{
				button_pressed(button);
				return;
			}
		}
	}

	bool UINpcTalk::answers(Buttons &left, Buttons &right) const
	{
		switch (type)
		{
			case TalkType::SENDYESNO:
				left = Buttons::YES;
				right = Buttons::NO;
				return true;
			case TalkType::SENDACCEPTDECLINE:
				left = Buttons::QYES;
				right = Buttons::QNO;
				return true;
			default:
				return false;
		}
	}

	void UINpcTalk::focus_answer(answer_focus::Focus f)
	{
		focus = f;

		Buttons left, right;

		if (!answers(left, right))
			return;

		buttons[left]->set_focused(focus == answer_focus::Focus::LEFT);
		buttons[right]->set_focused(focus == answer_focus::Focus::RIGHT);
	}

	UIElement::Type UINpcTalk::get_type() const
	{
		return TYPE;
	}

	UINpcTalk::TalkType UINpcTalk::get_by_value(int8_t value)
	{
		switch (value)
		{
			case 0:
				return TalkType::SENDOK;
			case 1:
				return TalkType::SENDYESNO;
			case 2:
				return TalkType::SENDGETTEXT;
			case 3:
				return TalkType::SENDGETNUMBER;
			case 4:
				return TalkType::SENDSIMPLE;
			case 12:
				return TalkType::SENDACCEPTDECLINE;
			default:
				return TalkType::NONE;
		}
	}

	// TODO: Move this to GraphicsGL?
	std::string UINpcTalk::format_text(const std::string &tx, const int32_t &npcid, std::vector<InlineImage> &images)
	{
		// The text renderer understands escaped line breaks, not literal LF/CR.
		// Quest prompts may contain both asset escapes and real newlines.
		std::string formatted_text;

		for (size_t i = 0; i < tx.size(); ++i)
		{
			if (tx[i] == '\r' || tx[i] == '\n')
			{
				if (tx[i] == '\r' && i + 1 < tx.size() && tx[i + 1] == '\n') ++i;
				formatted_text += "\\n";
			}
			else formatted_text += tx[i];
		}

		NpcTextMarkup::Lookups lookups {
			[]() { return Stage::get().get_player().get_name(); },
			[](int32_t id) { return std::string(nl::nx::string["Npc.img"][std::to_string(id)]["name"]); },
			[](int32_t id) { return ItemData::get(id).get_name(); },
			[](int32_t id) { return std::string(nl::nx::string["Mob.img"][std::to_string(id)]["name"]); },
			[](int32_t id) { return NxHelper::Map::get_map_info_by_id(id).name; }
		};

		std::vector<NpcTextMarkup::InlineImage> tags;
		formatted_text = NpcTextMarkup::expand(formatted_text, lookups, &tags);

		for (const auto &tag : tags)
		{
			InlineImage image;

			if (tag.tag == 'f' || tag.tag == 'F')
			{
				image.texture = path_image(tag.arg);
				image.size = image.texture.get_dimensions();
				image.anchor = image.texture.get_origin();
			} else
			{
				// Icons fill a 32x32 slot, their origin at its bottom left.
				int32_t id = tag.arg.size() <= 9 ? std::stoi(tag.arg) : 0;

				if (tag.tag == 's')
					image.texture = SkillData::get(id).get_icon(SkillData::Icon::NORMAL);
				else if (ItemData::get(id).is_valid())
					image.texture = ItemData::get(id).get_icon(false);

				if (image.texture.is_valid())
				{
					image.size = Point<int16_t>(32, 32);
					image.anchor = Point<int16_t>(0, 32);
				}
			}

			images.push_back(image);
		}

		return formatted_text;
	}

	void UINpcTalk::change_text(int32_t npcid, int8_t msgtype, int16_t style, int8_t speakerbyte, const std::string &tx,
								const std::map<int32_t, Texture> &headers, Menu menu)
	{
		local_responder = nullptr;
		message_type = msgtype;
		type = get_by_value(msgtype);
		const bool has_previous = (style & 0xff) != 0;
		const bool has_next = (style & 0xff00) != 0;
		if (msgtype == 0)
			type = has_previous ? TalkType::SENDNEXTPREV : has_next ? TalkType::SENDNEXT : TalkType::SENDOK;

		timestep = 0;
		draw_text = true;
		formatted_text_pos = 0;
		offset = 0;
		selected = 0;
		menu_kind = menu;
		images.clear();
		choices.clear();
		instant_text.clear();
		formatted_text = format_text(tx, npcid, images);

		std::vector<int32_t> inline_ids;

		if (type == TalkType::SENDSIMPLE && menu == Menu::LIST)
		{
			size_t first_link = formatted_text.find("#L");
			size_t link = first_link;
			int16_t space = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, " ").width();

			for (auto [id, label] : NpcTextMarkup::choices(formatted_text))
			{
				// Spaces before a choice indent it.
				size_t indent = 0;

				while (link != std::string::npos && indent < link && formatted_text[link - indent - 1] == ' ')
					indent++;

				link = formatted_text.find("#L", link + 2);

				// Listed choices are drawn without their images for now.
				label.erase(std::remove(label.begin(), label.end(), Text::BOX), label.end());
				auto header = headers.find(id);
				Choice choice { id, Point<int16_t>(static_cast<int16_t>(indent * space), 0), {},
					Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::DUSKBLUE, label, TEXT_WIDTH - LABEL_X) };

				if (header != headers.end())
					choice.header = header->second;

				choices.push_back(std::move(choice));
			}

			// The question is the text before the choices, less trailing
			// line breaks, colour codes and spaces.
			if (first_link != std::string::npos)
			{
				formatted_text.erase(first_link);

				while (size_t n = NpcTextMarkup::trailing_filler(formatted_text))
					formatted_text.erase(formatted_text.size() - n);

				images.resize(std::count(formatted_text.begin(), formatted_text.end(), Text::BOX));
			}
		} else if (type == TalkType::SENDSIMPLE)
		{
			formatted_text = NpcTextMarkup::mark_choices(formatted_text, inline_ids);
		}

		// Images and the marks around inline choices are all boxes to the
		// layout: a choice's start leaves room for its dot, its end takes none.
		std::vector<Point<int16_t>> boxes;
		std::vector<std::pair<size_t, size_t>> marks;
		size_t next_image = 0;

		for (size_t i = 0; i < formatted_text.size(); i++)
		{
			char &c = formatted_text[i];

			if (c == Text::BOX && next_image < images.size())
			{
				images[next_image].box = boxes.size();
				boxes.push_back(images[next_image++].size);
			} else if (c == NpcTextMarkup::CHOICE)
			{
				marks.emplace_back(boxes.size(), i);
				boxes.emplace_back(LABEL_X, 0);
				c = Text::BOX;
			} else if (c == NpcTextMarkup::CHOICE_END)
			{
				if (instant_text.size() < marks.size())
					instant_text.emplace_back(marks.back().second, i + 1);

				boxes.emplace_back(0, 0);
				c = Text::BOX;
			}
		}

		// Each #w turns showing at once on or off (FUN_0099dd54), as for the
		// rewards under a quest's last page.
		for (size_t on = formatted_text.find("#w"); on != std::string::npos;)
		{
			size_t off = formatted_text.find("#w", on + 2);
			size_t end = off == std::string::npos ? formatted_text.size() : off + 2;

			instant_text.emplace_back(on, end);
			on = off == std::string::npos ? off : formatted_text.find("#w", end);
		}

		std::sort(instant_text.begin(), instant_text.end());

		text = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::DARKGREY, formatted_text, TEXT_WIDTH);
		text.set_boxes(boxes);

		int16_t block = text.height();

		for (size_t i = 0; i < marks.size() && i < inline_ids.size(); i++)
			choices.push_back({ inline_ids[i], text.box_position(marks[i].first), text.box_position(marks[i].first + 1) });

		// Listed choices: 32 px rows under the question, a header above the
		// first of each group.
		if (menu == Menu::LIST && !choices.empty())
		{
			int16_t y = block + LIST_TOP;
			bool first_header = true;

			for (auto &choice : choices)
			{
				if (choice.header.is_valid())
				{
					if (!first_header)
						y += HEADER_GAP;

					first_header = false;
					choice.header_y = y + HEADER_Y;
					y = choice.header_y + choice.header.height() + HEADER_ROW;
				}

				choice.start.set_y(y + LINE);
				choice.end = choice.start + Point<int16_t>(LABEL_X + choice.label.width(), 0);
				y += ROW;
			}

			block = y;
		}

		if (speakerbyte == 0)
		{
			speaker = NxHelper::Npc::get_npc_node(npcid)["stand"]["0"];

			std::string namestr = nl::nx::string["Npc.img"][std::to_string(npcid)]["name"];
			name.change_text(namestr);
		} else
		{
			speaker = Texture();
			name.change_text("");
		}

		show_slider = block + PADDING > MAX_HEIGHT;

		if (show_slider)
		{
			height = MAX_HEIGHT;
			text_y = SCROLL_TEXT_Y;
			rowmax = (block + LINE - 1) / LINE;
			slider = Slider(Slider::Type::CLASSIC, Range<int16_t>(24, height + 14), 493, SCROLL_LINES, rowmax, onmoved);
		} else
		{
			height = std::max<int16_t>(block + PADDING, MIN_HEIGHT);
			text_y = 15 + (height - block) / 2;
			rowmax = 0;
		}

		for (auto &button : buttons)
		{
			button.second->set_active(false);
			button.second->set_state(Button::State::NORMAL);
			button.second->set_focused(false);
		}

		focus = answer_focus::Focus::NONE;

		// END CHAT, OK, YES/NO and ACCEPT/DECLINE sit on the bottom bar; NEXT
		// and BACK sit in the text area, clear of the scroll bar.
		int16_t bar_y = TOP + height + 32;
		int16_t text_button_y = TOP + height - 19;
		int16_t text_button_x = show_slider ? -8 : 0;

		auto place = [&](Buttons button, int16_t x, int16_t y)
		{
			buttons[button]->set_position(Point<int16_t>(x, y));
			buttons[button]->set_active(true);
		};

		place(Buttons::CLOSE, 9, bar_y);

		switch (type)
		{
			case TalkType::SENDOK:
				place(Buttons::OK, 475, bar_y);
				break;
			case TalkType::SENDYESNO:
				place(Buttons::YES, 421, bar_y);
				place(Buttons::NO, 475, bar_y);
				break;
			case TalkType::SENDACCEPTDECLINE:
				place(Buttons::QYES, 399, bar_y);
				place(Buttons::QNO, 464, bar_y);
				break;
			case TalkType::SENDNEXT:
			case TalkType::SENDNEXTPREV:
				if (has_previous)
					place(Buttons::PREV, 397 + text_button_x, text_button_y);

				if (has_next)
					place(Buttons::NEXT, 451 + text_button_x, text_button_y);
				else
					place(Buttons::OK, 475, bar_y);
				break;
			case TalkType::SENDSIMPLE:
				// A server's menu has no NEXT: clicking a choice answers.
				if (menu == Menu::LIST)
					place(Buttons::NEXT, 451 + text_button_x, text_button_y);
				break;
			case TalkType::SENDGETTEXT:
			case TalkType::SENDGETNUMBER:
			default:
				break;
		}

		int16_t total = TOP + height + bottom.height();

		position = Point<int16_t>(
			Constants::Constants::get().get_viewwidth() / 2 - top.width() / 2,
			Constants::Constants::get().get_viewheight() / 2 - total / 2
		);
		dimension = Point<int16_t>(top.width(), total);
	}
}
