#include "UIFieldClock.h"

#include "../../Constants.h"
#include "../../Gameplay/MapleMap/ClockRules.h"
#include "../../Gameplay/Stage.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		nl::node timer()
		{
			return nl::nx::map["Obj"]["etc.img"]["timer"];
		}

		nl::node font()
		{
			return nl::nx::map["Obj"]["etc.img"]["clock"]["fontTime"];
		}
	}

	UIClockWindow::UIClockWindow(int32_t seconds)
			: UIElement(Point<int16_t>(), Texture(timer()["backgrnd"]).get_dimensions()), reading(seconds),
			  running(false), started(ContinuousTimer::get().start()), from(seconds)
	{
		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
	}

	void UIClockWindow::draw(float) const
	{
		countdown::Face face = countdown::face(reading);

		Texture(timer()[face.hours ? "backgrndhour" : "backgrnd"]).draw(position);

		nl::node digits = font();

		for (size_t i = 0; i < face.digits.size(); i++)
			if (face.digits[i] >= 0)
				Texture(digits[std::to_string(face.digits[i])])
						.draw(position + Point<int16_t>(countdown::DIGIT_X[i], countdown::DIGIT_Y));
	}

	void UIClockWindow::update()
	{
		UIElement::update();

		if (running)
		{
			int64_t left_ms = from * 1000LL - ContinuousTimer::get().stop(started) / 1000;
			reading = countdown::seconds_left(left_ms);
		}
	}

	void UIClockWindow::update_screen(int16_t new_width, int16_t)
	{
		position = Point<int16_t>((new_width - countdown::WIDTH) / 2, countdown::TOP);
	}

	bool UIClockWindow::is_in_range(Point<int16_t>) const
	{
		return false;
	}

	void UIClockWindow::start()
	{
		if (running)
			return;

		running = true;
		started = ContinuousTimer::get().start();
		from = reading;
	}

	bool UIClockWindow::is_running() const
	{
		return running;
	}

	int32_t UIClockWindow::get_reading() const
	{
		return reading;
	}

	UIFieldClock::UIFieldClock(int32_t seconds, bool sw) : UIClockWindow(seconds), switched(sw)
	{
		start();
	}

	bool UIFieldClock::is_switched() const
	{
		return switched;
	}

	void UIFieldClock::unswitch()
	{
		switched = false;
	}

	UIElement::Type UIFieldClock::get_type() const
	{
		return TYPE;
	}

	UIEventClock::UIEventClock(int32_t seconds) : UIClockWindow(seconds)
	{}

	void UIEventClock::update()
	{
		UIClockWindow::update();

		// Off the event's maps, or once it reads 0, it is taken down.
		if (!countdown::stays_on(Stage::get().get_mapid()) || get_reading() == 0)
			deactivate();
	}

	UIElement::Type UIEventClock::get_type() const
	{
		return TYPE;
	}
}
