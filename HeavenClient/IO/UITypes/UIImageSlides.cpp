#include "UIImageSlides.h"

#include "../KeyAction.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Util/NxFiles.h"
#include "../../Util/PathImage.h"

#include <algorithm>

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// FUN_009a3cae and FUN_009acc20 for type 9: 300 high inside, plus the 86 of the button rows.
		constexpr Point<int16_t> DIMENSION = Point<int16_t>(456, 386);
		// From the window's top left (FUN_009a9f68, FUN_009aa346).
		constexpr Point<int16_t> PREV_BUTTON = Point<int16_t>(324, 309);
		constexpr Point<int16_t> NEXT_BUTTON = Point<int16_t>(378, 309);
		constexpr Point<int16_t> OK_BUTTON = Point<int16_t>(402, 360);
		constexpr Point<int16_t> CLOSE_BUTTON = Point<int16_t>(9, 360);
		// FUN_004eda94 keeps NPC dialogs on the 800 by 600 screen.
		constexpr int16_t SCREEN_WIDTH = 800;
		constexpr int16_t SCREEN_HEIGHT = 600;

		Point<int16_t> centred()
		{
			int16_t x = (Constants::Constants::get().get_viewwidth() - DIMENSION.x()) / 2;
			int16_t y = (Constants::Constants::get().get_viewheight() - DIMENSION.y()) / 2;

			return Point<int16_t>(std::clamp<int16_t>(x, 0, SCREEN_WIDTH - DIMENSION.x()),
								  std::clamp<int16_t>(y, 0, SCREEN_HEIGHT - DIMENSION.y()));
		}
	}

	UIImageSlides::UIImageSlides(const std::vector<std::string> &paths)
			: UIElement(centred(), DIMENSION), background(DIMENSION.x(), DIMENSION.y(), Color::Name::WHITE, 1.0f), page(0),
			  shown(image_slides::opened(paths.size()))
	{
		nl::node UtilDlgEx = NxFiles::ui_classic["UIWindow.img"]["UtilDlgEx"];

		buttons[Buttons::PREV] = std::make_unique<MapleButton>(UtilDlgEx["BtPrev"], PREV_BUTTON);
		buttons[Buttons::NEXT] = std::make_unique<MapleButton>(UtilDlgEx["BtNext"], NEXT_BUTTON);
		buttons[Buttons::OK] = std::make_unique<MapleButton>(UtilDlgEx["BtOK"], OK_BUTTON);
		buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(UtilDlgEx["BtClose"], CLOSE_BUTTON);

		// FUN_009ab124 looks each path up in the game data; one that names no image shows nothing.
		for (const std::string &path : image_slides::pages(paths))
			pages.push_back(path_image(path));

		show();
	}

	void UIImageSlides::draw(float alpha) const
	{
		background.draw(position);

		// At the window's top left, less the image's origin; a page past the last (empty paths) shows nothing.
		if (page < pages.size())
			pages[page].draw(position);

		UIElement::draw_buttons(alpha);
	}

	void UIImageSlides::send_key(int32_t keycode, bool pressed, bool escape)
	{
		// FUN_009ac0b6: Escape is Close and Enter ends the dialog on any page; neither turns it, nor do the arrows.
		if (pressed && (escape || keycode == KeyAction::Id::RETURN))
			deactivate();
	}

	UIElement::Type UIImageSlides::get_type() const
	{
		return TYPE;
	}

	Button::State UIImageSlides::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::PREV:
				page--;
				shown = image_slides::turned(shown, page, pages.size());
				show();
				break;
			case Buttons::NEXT:
				page++;
				shown = image_slides::turned(shown, page, pages.size());
				show();
				break;
			case Buttons::OK:
			case Buttons::CLOSE:
				deactivate();
				break;
		}

		return Button::State::NORMAL;
	}

	void UIImageSlides::show()
	{
		buttons[Buttons::PREV]->set_active(shown.prev);
		buttons[Buttons::NEXT]->set_active(shown.next);
		buttons[Buttons::OK]->set_active(shown.ok());
	}
}
