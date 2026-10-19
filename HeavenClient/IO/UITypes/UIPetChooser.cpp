#include "UIPetChooser.h"

#include "../KeyAction.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// FUN_009a3cae and FUN_009a3ced; FUN_009a3e38 centres it on the screen.
		constexpr int16_t WIDTH = 419, HEIGHT = 297;
		// The rest is from the window's top left (FUN_009a9b25 and FUN_009a3e9f). v83 draws a line's cap top
		// about 3 below where it is drawn, we 9, so text goes up by the difference.
		constexpr int16_t TEXT_RAISE = 6;
		const Point<int16_t> PREV_AT(15, 191);
		const Point<int16_t> NEXT_AT(341, 191);
		const Point<int16_t> OK_AT(314, 272);
		const Point<int16_t> CANCEL_AT(363, 272);
		const Point<int16_t> EXIT_AT(9, 272);
		const Point<int16_t> NAME_TAG_AT(123, 220);
		const Point<int16_t> PET_AT(209, 218);
		constexpr int16_t NAME_X = 210, NAME_Y = 224;
		constexpr int16_t LEVEL_X = 194, CLOSENESS_X = 280, NUMBERS_Y = 243;
		// The caption's box (FUN_009acc20): its text starts 120 across, and is centred on 100 rows from 28 down
		// when it fits. The box is 280 wide, and like the choice of styles' text it keeps 10 px clear on each
		// side.
		constexpr int16_t CAPTION_X = 120, CAPTION_TOP = 28, CAPTION_ROWS = 100;
		constexpr uint16_t CAPTION_WIDTH = 260;
	}

	UIPetChooser::UIPetChooser(std::string text, std::vector<Pet> p, std::function<void(size_t)> oc,
							   std::function<void()> ocancel)
			: UIElement(Point<int16_t>(), Point<int16_t>(WIDTH, HEIGHT)), pets(std::move(p)), onchoose(std::move(oc)),
			  oncancel(std::move(ocancel)), shown(0)
	{
		nl::node window = NxFiles::ui_classic["UIWindow.img"];
		nl::node avatar = window["UtilDlgEx_Avatar"];
		nl::node basic = NxFiles::ui_classic["Basic.img"];

		sprites.emplace_back(window["UtilDlgEx_Pet"]["backgrnd"]);
		name_tag = avatar["nameTag"];

		buttons[Buttons::PREV] = std::make_unique<MapleButton>(avatar["BtPrev"], PREV_AT);
		buttons[Buttons::NEXT] = std::make_unique<MapleButton>(avatar["BtNext"], NEXT_AT);
		buttons[Buttons::OK] = std::make_unique<MapleButton>(basic["BtOK2"], OK_AT);
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(basic["BtCancel2"], CANCEL_AT);
		buttons[Buttons::EXIT] = std::make_unique<MapleButton>(avatar["BtExit"], EXIT_AT);

		caption = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, text, CAPTION_WIDTH);
		name = Text(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::BLACK);
		level = Text(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::BLACK);
		closeness = Text(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::BLACK);

		show(0);
		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
	}

	void UIPetChooser::draw(float alpha) const
	{
		UIElement::draw_sprites(alpha);

		int16_t height = caption.height();
		int16_t top = height <= CAPTION_ROWS ? CAPTION_TOP + (CAPTION_ROWS - 6 - height) / 2 : CAPTION_TOP - 6;

		caption.draw(position + Point<int16_t>(CAPTION_X, top + 2 - TEXT_RAISE));
		name_tag.draw(position + NAME_TAG_AT);
		name.draw(position + Point<int16_t>(NAME_X, NAME_Y - TEXT_RAISE));
		level.draw(position + Point<int16_t>(LEVEL_X, NUMBERS_Y - TEXT_RAISE));
		closeness.draw(position + Point<int16_t>(CLOSENESS_X, NUMBERS_Y - TEXT_RAISE));
		standing.draw(position + PET_AT, alpha);

		UIElement::draw_buttons(alpha);
	}

	void UIPetChooser::update()
	{
		UIElement::update();

		standing.update();
	}

	void UIPetChooser::update_screen(int16_t new_width, int16_t new_height)
	{
		int16_t x = std::clamp<int16_t>(new_width / 2 - WIDTH / 2, 0, std::max<int16_t>(new_width - WIDTH, 0));
		int16_t y = std::clamp<int16_t>(new_height / 2 - HEIGHT / 2, 0, std::max<int16_t>(new_height - HEIGHT, 0));

		position = Point<int16_t>(x, y);
	}

	void UIPetChooser::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		// FUN_009ac0b6: Escape is Cancel, and FUN_004edd21 makes Enter OK. The arrows do nothing here.
		if (escape)
			cancel();
		else if (keycode == KeyAction::Id::RETURN)
			choose();
	}

	UIElement::Type UIPetChooser::get_type() const
	{
		return TYPE;
	}

	Button::State UIPetChooser::button_pressed(uint16_t buttonid)
	{
		size_t count = pets.size();

		switch (buttonid)
		{
			case Buttons::OK:
				choose();
				break;
			case Buttons::CANCEL:
			case Buttons::EXIT:
				cancel();
				break;
			case Buttons::PREV:
				show((shown + count - 1) % count);
				break;
			case Buttons::NEXT:
				show((shown + 1) % count);
				break;
			default:
				break;
		}

		return Button::State::NORMAL;
	}

	void UIPetChooser::show(size_t index)
	{
		shown = index;

		const Pet &pet = pets[index];

		standing = nl::nx::item["Pet"][std::to_string(pet.itemid) + ".img"]["stand0"];
		name.change_text(pet.name);
		level.change_text(std::to_string(pet.level));
		closeness.change_text(std::to_string(pet.closeness));
	}

	void UIPetChooser::choose()
	{
		deactivate();
		onchoose(shown);
	}

	void UIPetChooser::cancel()
	{
		deactivate();

		if (oncancel)
			oncancel();
	}
}
