#include "UILieDetector.h"

#include "../KeyAction.h"

#include "../Components/MapleButton.h"

#include "../../Audio/Audio.h"
#include "../../Constants.h"
#include "../../Gameplay/LieDetector.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/InventoryPackets.h"

#include <chrono>

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		int64_t now_ms()
		{
			using namespace std::chrono;

			return duration_cast<milliseconds>(steady_clock::now().time_since_epoch()).count();
		}

		Point<int16_t> centred(Point<int16_t> dimension)
		{
			return Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);
		}

		// v83 draws a line's cap top about 3 below where it is drawn, we 9, so text goes up by the difference.
		constexpr int16_t TEXT_RAISE = 6;

		// The name prompt's white box and the grey strip under it, in the art UIClassic.nx has.
		const Rectangle<int16_t> NAME_BOX = Rectangle<int16_t>(80, 270, 91, 106);
		constexpr size_t MAX_NAME = 12;
		constexpr Point<int16_t> NAME_OK = Point<int16_t>(182, 134);
		constexpr Point<int16_t> NAME_CANCEL = Point<int16_t>(234, 134);

		// The challenge's art in UIClassic.nx has the picture's box at (98, 136), 196 by 43, the answer's
		// white box under it and the OK button's grey strip at the bottom, where the 2025 client puts the
		// picture at (57, 106), the edit at (62, 171) and OK at (228, 213) (report c025).
		const Rectangle<int16_t> PICTURE_BOX = Rectangle<int16_t>(98, 294, 136, 179);
		const Rectangle<int16_t> ANSWER_BOX = Rectangle<int16_t>(101, 290, 186, 201);
		constexpr size_t MAX_ANSWER = 12;
		constexpr Point<int16_t> CHALLENGE_OK = Point<int16_t>(258, 227);
		// Macro/num1's digits for "mm,ss" in the clock's box, as 0x7e6a94 draws them.
		constexpr std::array<int16_t, 4> CLOCK_X = { 111, 132, 168, 189 };
		constexpr int16_t COMMA_X = 155;
		constexpr int16_t CLOCK_Y = 32;
		// Where the 2025 art says "Number of Tries Left :", which UIClassic's lacks.
		constexpr Point<int16_t> TRIES_AT = Point<int16_t>(30, 228);
		// The challenge sits at the screen's top right (0x7e65de: 760 less its width, 10).
		constexpr int16_t CHALLENGE_RIGHT = 40;
		constexpr int16_t CHALLENGE_TOP = 10;

		// FUN_007e8a2f: the face at (20, 20), lines from x 87 centred on y 59, OK at (222, 125).
		constexpr Point<int16_t> FACE_AT = Point<int16_t>(20, 20);
		constexpr int16_t RESULT_LEFT = 87;
		constexpr int16_t RESULT_MIDDLE = 59;
		constexpr uint16_t RESULT_WIDTH = 185;
		constexpr Point<int16_t> RESULT_OK = Point<int16_t>(222, 125);
	}

	UILieDetectorName::UILieDetectorName(std::function<void(const std::string &)> os)
			: UIElement(Point<int16_t>(), Point<int16_t>()), onsend(std::move(os))
	{
		nl::node basic = NxFiles::ui_classic["Basic.img"];
		Texture background = NxFiles::ui_classic["UIWindow.img"]["Macro"]["backgrnd"];
		sprites.emplace_back(NxFiles::ui_classic["UIWindow.img"]["Macro"]["backgrnd"]);

		buttons[Buttons::OK] = std::make_unique<MapleButton>(basic["BtOK2"], NAME_OK);
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(basic["BtCancel2"], NAME_CANCEL);

		field = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, NAME_BOX, MAX_NAME);
		field.set_enter_callback(
				[&](std::string)
				{
					send();
				}
		);
		field.set_key_callback(
				KeyAction::Id::ESCAPE,
				[&]()
				{
					close();
				}
		);

		dimension = background.get_dimensions();
		position = centred(dimension);

		field.set_state(Textfield::State::FOCUSED);
	}

	void UILieDetectorName::draw(float alpha) const
	{
		draw_sprites(alpha);

		field.draw(position - Point<int16_t>(0, TEXT_RAISE));

		UIElement::draw_buttons(alpha);
	}

	void UILieDetectorName::update()
	{
		UIElement::update();

		field.update(position);
	}

	Cursor::State UILieDetectorName::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State state = field.send_cursor(cursorpos, clicked);

		if (state != Cursor::State::IDLE)
			return state;

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UILieDetectorName::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
			close();
		else if (keycode == KeyAction::Id::RETURN)
			send();
	}

	UIElement::Type UILieDetectorName::get_type() const
	{
		return TYPE;
	}

	Button::State UILieDetectorName::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::OK)
			send();
		else
			close();

		return Button::State::NORMAL;
	}

	void UILieDetectorName::send()
	{
		std::string name = field.get_text();

		close();

		if (!name.empty())
			onsend(name);
	}

	void UILieDetectorName::close()
	{
		field.set_state(Textfield::State::DISABLED);
		deactivate();
	}

	UILieDetector::UILieDetector(PixelTexture pic, int32_t time_left, bool first, bool by_skill)
			: UIElement(Point<int16_t>(), Point<int16_t>()), picture(std::move(pic)), deadline(now_ms() + time_left)
	{
		nl::node macro = NxFiles::ui_classic["UIWindow.img"]["Macro"];
		// Dialog B's art has dialog A's layout, though UIClassic.nx left its words in Korean.
		nl::node art = by_skill ? NxFiles::ui_classic["UIWindow.img"]["MacroSkill"]["backgrnd2"] : macro["backgrnd4"];
		Texture background = art;
		sprites.emplace_back(art);

		for (size_t i = 0; i < digits.size(); i++)
			digits[i] = macro["num1"][std::to_string(i)];

		comma = macro["num1"]["comma"];

		buttons[Buttons::OK] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtOK2"], CHALLENGE_OK);

		field = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, ANSWER_BOX, MAX_ANSWER);
		field.set_enter_callback(
				[&](std::string)
				{
					send();
				}
		);

		tries = Text(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::BLACK,
					 "Number of Tries Left : " + std::to_string(lie_detector::tries(first)));

		dimension = background.get_dimensions();
		position = Point<int16_t>(Constants::Constants::get().get_viewwidth() - CHALLENGE_RIGHT - dimension.x(),
								  CHALLENGE_TOP);

		field.set_state(Textfield::State::FOCUSED);
	}

	void UILieDetector::draw(float alpha) const
	{
		draw_sprites(alpha);

		Point<int16_t> size = picture.get_dimensions();
		Point<int16_t> box = Point<int16_t>(PICTURE_BOX.left() + (PICTURE_BOX.width() - size.x()) / 2,
											PICTURE_BOX.top() + (PICTURE_BOX.height() - size.y()) / 2);
		picture.draw(position + box);

		if (auto clock = lie_detector::clock(static_cast<int32_t>(deadline - now_ms())))
		{
			for (size_t i = 0; i < clock->size(); i++)
				digits[(*clock)[i]].draw(position + Point<int16_t>(CLOCK_X[i], CLOCK_Y));

			comma.draw(position + Point<int16_t>(COMMA_X, CLOCK_Y));
		}

		tries.draw(position + TRIES_AT);
		field.draw(position - Point<int16_t>(0, TEXT_RAISE));

		UIElement::draw_buttons(alpha);
	}

	void UILieDetector::update()
	{
		UIElement::update();

		field.update(position);
	}

	Cursor::State UILieDetector::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State state = field.send_cursor(cursorpos, clicked);

		if (state != Cursor::State::IDLE)
			return state;

		return UIElement::send_cursor(clicked, cursorpos);
	}

	UIElement::Type UILieDetector::get_type() const
	{
		return TYPE;
	}

	Button::State UILieDetector::button_pressed(uint16_t)
	{
		send();

		return Button::State::NORMAL;
	}

	// 0x7e7603: nothing typed sends nothing; else the time left is kept for the next picture, the answer
	// goes, and the challenge closes.
	void UILieDetector::send()
	{
		std::string answer = field.get_text();

		if (answer.empty())
			return;

		int32_t left = static_cast<int32_t>(deadline - now_ms());
		// 0 would mean no test.
		lie_detector::time_left = left == 0 ? -1 : left;

		AnswerLieDetectorPacket(answer).dispatch();

		field.set_state(Textfield::State::DISABLED);
		deactivate();
	}

	UILieDetectorResult::UILieDetectorResult(int portrait, const std::vector<std::string> &lines)
			: UIElement(Point<int16_t>(), Point<int16_t>())
	{
		// UIClassic's Macro/backgrnd3 is a name prompt, so both codes take MacroSkill's plain one.
		Texture background = NxFiles::ui_classic["UIWindow.img"]["MacroSkill"]["backgrnd3"];
		sprites.emplace_back(NxFiles::ui_classic["UIWindow.img"]["MacroSkill"]["backgrnd3"]);
		sprites.emplace_back(NxFiles::ui_classic["UIWindow.img"]["Macro"]["admin"][std::to_string(portrait)],
							 FACE_AT);

		buttons[Buttons::OK] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtOK2"], RESULT_OK);

		std::string joined;

		for (const std::string &line : lines)
			joined += (joined.empty() ? "" : "\\n") + line;

		text = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, joined, RESULT_WIDTH);

		dimension = background.get_dimensions();
		position = centred(dimension);
	}

	void UILieDetectorResult::draw(float alpha) const
	{
		draw_sprites(alpha);

		text.draw(position + Point<int16_t>(RESULT_LEFT, RESULT_MIDDLE - text.height() / 2 - TEXT_RAISE));

		UIElement::draw_buttons(alpha);
	}

	void UILieDetectorResult::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed && (escape || keycode == KeyAction::Id::RETURN))
			deactivate();
	}

	UIElement::Type UILieDetectorResult::get_type() const
	{
		return TYPE;
	}

	Button::State UILieDetectorResult::button_pressed(uint16_t)
	{
		deactivate();

		return Button::State::NORMAL;
	}
}
