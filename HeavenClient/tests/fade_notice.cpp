#include "../IO/Components/FadeNotice.h"

#include <cassert>
#include <vector>

int main()
{
	// Every notice stacks above the ones made before it; when one goes, the
	// later ones move down, and moving a notice keeps its place.
	{
		std::vector<ms::FadeNotice> line;
		line.emplace_back(1000);
		line.emplace_back(1000);
		line.emplace_back(1000);
		ms::FadeNotice other(1000);
		assert(line[0].raised() == 0 && line[2].raised() == 10 && other.raised() == 15);

		line.erase(line.begin() + 1);
		assert(line[0].raised() == 0 && line[1].raised() == 5 && other.raised() == 10);

		line.erase(line.begin());
		assert(line[0].raised() == 0 && other.raised() == 5);
	}

	assert(ms::FadeNotice(1000).raised() == 0);

	// Untouched: a second in, until 30 s from being made, a second out.
	ms::FadeNotice notice(30000);
	assert(notice.opacity() == 0.0f);
	notice.update(500);
	assert(notice.opacity() == 0.5f);
	notice.update(501);
	assert(notice.opacity() == 1.0f && !notice.closed());
	notice.update(28999);
	assert(notice.opacity() == 1.0f && !notice.closed());
	notice.update(1);
	assert(notice.closed() && notice.opacity() == 1.0f);
	notice.update(500);
	assert(notice.opacity() == 0.5f && !notice.gone());
	notice.update(501);
	assert(notice.gone() && notice.opacity() == 0.0f);

	// Closed while showing: fades out at once.
	ms::FadeNotice pressed(30000);
	pressed.update(2000);
	pressed.close();
	assert(pressed.closed());
	pressed.update(8);
	assert(pressed.opacity() == 1.0f);
	pressed.update(1001);
	assert(pressed.gone());

	// Closed while fading in: finishes fading in first.
	ms::FadeNotice early(30000);
	early.update(200);
	early.close();
	early.update(300);
	assert(early.opacity() == 0.5f && !early.gone());
	early.update(501);
	early.update(8);
	early.update(1001);
	assert(early.gone());

	// Made while LOCK_UI is on: waits unseen, then fades in.
	ms::FadeNotice held(30000);
	held.update(5000, true);
	assert(held.waiting() && held.opacity() == 0.0f);
	held.update(500, false);
	assert(!held.waiting() && held.opacity() == 0.5f);

	// Locked while shown: fades out, waits, and on unlock fades in with its
	// whole time again from the lock.
	ms::FadeNotice shown(30000);
	shown.update(20000);
	shown.update(10, true);
	assert(shown.waiting() && shown.opacity() == 1.0f);
	shown.update(500, true);
	assert(shown.opacity() == 0.5f);
	shown.update(60000, true);
	assert(shown.waiting() && shown.opacity() == 0.0f && !shown.gone());
	shown.update(1001, false);
	assert(shown.opacity() == 1.0f && !shown.closed());
	// Its time ran out 30 s after the lock, while it waited.
	shown.update(8, false);
	assert(shown.closed());

	// A short lock: the 30 s count from the lock at 20010.
	ms::FadeNotice brief(30000);
	brief.update(20000);
	brief.update(10, true);
	brief.update(2000, true);
	brief.update(1001, false);
	assert(brief.opacity() == 1.0f);
	brief.update(50010 - 23011, false);
	assert(!brief.closed());
	brief.update(1, false);
	assert(brief.closed());

	return 0;
}
