#include "UIGuildMarkMaker.h"

#include "UINotice.h"

#include "../KeyAction.h"
#include "../UI.h"

#include "../Components/MapleButton.h"
#include "../Components/MapleComboBox.h"

#include "../../Constants.h"
#include "../../Net/Packets/GameplayPackets.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <memory>
#include <string>
#include <vector>

namespace ms
{
	namespace
	{
		constexpr Point<int16_t> SIZE = Point<int16_t>(250, 343);
		// Where the preview goes, over the sun.
		constexpr Point<int16_t> PREVIEW = Point<int16_t>(117, 103);
		constexpr Point<int16_t> COMBO = Point<int16_t>(36, 252);
		// The box's middle repeats in 5-pixel pieces: this makes the box 88 pixels wide.
		constexpr int64_t COMBO_WIDTH = 66;
		constexpr int16_t WAIT_MS = 1000;

		// Each box's top left; the first two hold the background's arrows, the others the mark's.
		constexpr std::array<Point<int16_t>, 4> BOXES = {
			Point<int16_t>(34, 151), Point<int16_t>(127, 151), Point<int16_t>(34, 207), Point<int16_t>(127, 207)
		};
		// "Create Guild Emblem", then a label for each box.
		constexpr std::array<Point<int16_t>, 5> MESSAGES = {
			Point<int16_t>(74, 44), Point<int16_t>(56, 153), Point<int16_t>(148, 153), Point<int16_t>(56, 209),
			Point<int16_t>(148, 209)
		};

		// The names of a node's children.
		std::vector<std::string> names(nl::node node)
		{
			std::vector<std::string> names;

			for (auto child : node)
				names.push_back(child.name());

			return names;
		}
	}

	UIGuildMarkMaker::UIGuildMarkMaker(const GuildEmblem& current) : UIElement(Point<int16_t>(0, 0), SIZE),
		picked(emblem::initial(current)), last{}, step(Step::UNROLL), wait(WAIT_MS)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["UserList"]["Guild"]["MakeMark"];

		backgrnd = src["backgrnd"];
		backgrnd.play(Animation::End::HOLD);
		inner = src["inner"];

		for (size_t i = 0; i < messages.size(); i++)
			messages[i] = src["message" + std::to_string(i)];

		blank = ColorBox(GuildMark::SIZE, GuildMark::SIZE, Color::Name::WHITE, 1.0f);

		nl::node marks = NxFiles::ui_classic["GuildMark.img"];
		last[1] = emblem::last_id(names(marks["BackGround"]));

		for (uint16_t category : { 2, 3, 4, 5, 9 })
			last[category] = emblem::last_id(names(marks["Mark"][guild_mark_category(category * 1000)]));

		// Each box has a left and a right arrow.
		for (uint16_t box = 0; box < BOXES.size(); box++)
		{
			Point<int16_t> at = BOXES[box] + Point<int16_t>(3, 21);

			buttons[2 * box] = std::make_unique<MapleButton>(src["BtLeft"], at);
			buttons[2 * box + 1] = std::make_unique<MapleButton>(src["BtRight"], at + Point<int16_t>(43, 0));
		}

		buttons[Buttons::AGREE] = std::make_unique<MapleButton>(src["BtAgree"], Point<int16_t>(82, 288));
		buttons[Buttons::DISAGREE] = std::make_unique<MapleButton>(src["BtDisagree"], Point<int16_t>(133, 288));

		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());

		for (auto& button : buttons)
			button.second->set_state(Button::State::DISABLED);

		show(picked);
	}

	void UIGuildMarkMaker::draw(float alpha) const
	{
		backgrnd.draw(DrawArgument(position), alpha);

		// The preview is there from the start (FUN_00595163), the rest once the controls work.
		if (step == Step::READY)
		{
			for (auto box : BOXES)
				inner.draw(position + box + inner.get_origin());

			for (size_t i = 0; i < messages.size(); i++)
				messages[i].draw(position + MESSAGES[i]);
		}

		blank.draw(position + PREVIEW);
		preview.draw(position + PREVIEW);

		if (step == Step::READY)
			UIElement::draw_buttons(alpha);
	}

	void UIGuildMarkMaker::update()
	{
		UIElement::update();

		backgrnd.update();

		switch (step)
		{
		case Step::UNROLL:
			if (!backgrnd.is_playing())
				step = Step::WAIT;

			break;
		case Step::WAIT:
			wait -= Constants::TIMESTEP;

			if (wait <= 0)
				set_ready();

			break;
		case Step::READY:
		{
			// The combo has no callback; a new category starts at its first mark (FUN_0059540e).
			uint16_t category = buttons[Buttons::CATEGORY]->get_selected();

			if (category != emblem::category_index(picked.mark))
			{
				picked.mark = emblem::category_mark(category);
				show(picked);
			}

			break;
		}
		}
	}

	void UIGuildMarkMaker::update_screen(int16_t new_width, int16_t new_height)
	{
		// Centred across, and as high as on an 800 by 600 screen.
		Point<int16_t> place = Point<int16_t>(
			std::max<int16_t>((new_width - SIZE.x()) / 2, 0),
			std::max<int16_t>(new_height / 2 - 200, 0)
		);

		if (place == position && buttons.count(Buttons::CATEGORY))
			return;

		position = place;

		// The combo keeps the place it was made at.
		place_category();
	}

	Cursor::State UIGuildMarkMaker::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		auto& combo = buttons[Buttons::CATEGORY];

		// The open list covers OK: a pick, which closes the list, mustn't press OK under it too.
		if (combo->is_pressed() && combo->in_combobox(cursorpos))
		{
			Cursor::State state = combo->send_cursor(clicked, cursorpos);

			return clicked && state == Cursor::State::IDLE ? Cursor::State::CLICKING : state;
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIGuildMarkMaker::send_key(int32_t keycode, bool pressed, bool escape)
	{
		// Nothing works until the controls do.
		if (!pressed || step != Step::READY)
			return;

		if (escape)
			deactivate();
		else if (keycode == KeyAction::Id::RETURN)
			agree();
	}

	UIElement::Type UIGuildMarkMaker::get_type() const
	{
		return TYPE;
	}

	void UIGuildMarkMaker::save()
	{
		deactivate();
		SetGuildEmblemPacket(picked).dispatch();
	}

	Button::State UIGuildMarkMaker::button_pressed(uint16_t buttonid)
	{
		auto& combo = buttons[Buttons::CATEGORY];

		if (buttonid == Buttons::CATEGORY)
		{
			combo->toggle_pressed();

			return Button::State::NORMAL;
		}

		if (combo->is_pressed())
			combo->toggle_pressed();

		switch (buttonid)
		{
		case Buttons::BACKGROUND_LEFT:
		case Buttons::BACKGROUND_RIGHT:
			picked.background = emblem::step_background(picked.background, buttonid == Buttons::BACKGROUND_RIGHT, last);
			break;
		case Buttons::BACKGROUND_COLOR_LEFT:
		case Buttons::BACKGROUND_COLOR_RIGHT:
			picked.background_color = emblem::step_color(picked.background_color,
														  buttonid == Buttons::BACKGROUND_COLOR_RIGHT);
			break;
		case Buttons::MARK_LEFT:
		case Buttons::MARK_RIGHT:
			picked.mark = emblem::step_mark(picked.mark, buttonid == Buttons::MARK_RIGHT, last);
			break;
		case Buttons::MARK_COLOR_LEFT:
		case Buttons::MARK_COLOR_RIGHT:
			picked.mark_color = emblem::step_color(picked.mark_color, buttonid == Buttons::MARK_COLOR_RIGHT);
			break;
		case Buttons::AGREE:
			agree();
			return Button::State::NORMAL;
		case Buttons::DISAGREE:
			deactivate();
			return Button::State::NORMAL;
		default:
			return Button::State::NORMAL;
		}

		show(picked);

		return Button::State::NORMAL;
	}

	void UIGuildMarkMaker::place_category()
	{
		// Indented as in the StringPool (0xcf3 to 0xcf7).
		std::vector<std::string> categories = {
			"      Animal", "      Plant", "      Pattern", "      Character", "      ETC"
		};

		buttons[Buttons::CATEGORY] = std::make_unique<MapleComboBox>(
			MapleComboBox::Type::DEFAULT, categories, emblem::category_index(picked.mark), position, COMBO, COMBO_WIDTH
		);

		if (step != Step::READY)
			buttons[Buttons::CATEGORY]->set_state(Button::State::DISABLED);
	}

	void UIGuildMarkMaker::set_ready()
	{
		step = Step::READY;

		for (auto& button : buttons)
			button.second->set_state(Button::State::NORMAL);
	}

	void UIGuildMarkMaker::show(const GuildEmblem& emblem)
	{
		preview = GuildMark(emblem);
	}

	void UIGuildMarkMaker::agree()
	{
		UI::get().emplace<UIYesNo>(
			"Would you like to save it as a guild emblem?",
			[](bool yes)
			{
				auto maker = UI::get().get_element<UIGuildMarkMaker>();

				// No leaves the maker as it was.
				if (yes && maker && maker->is_active())
					maker->save();
			}
		);
	}
}
